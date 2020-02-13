/* eslint-disable @typescript-eslint/no-explicit-any */
import * as R from 'ramda'
import { Observable, ReplaySubject, Scheduler, Subject, Subscription as RxSubscription } from 'rxjs'
import { catchError, tap } from 'rxjs/operators'
import {
  CommandApi,
  CommandResult,
  FishName,
  FishType,
  FishTypeImpl,
  PondObservables,
  Source,
  StateEffect,
} from '.'
import { EventStore } from './eventstore'
import { AllEventsSortOrders, Event, OffsetMap } from './eventstore/types'
import { intoOrderedChunks } from './eventstore/utils'
import { CommandExecutor } from './executors/commandExecutor'
import { FishEventStore, FishInfo } from './fishEventStore'
import log from './loggers'
import { SendToStore } from './pond'
import { PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { EnvelopeFromStore } from './store/util'
import { Subscription, SubscriptionSet, subscriptionsToEnvelopePredicate } from './subscription'
import { ObserveMethod, Psn, Semantics, StateWithProvenance, Timestamp } from './types'
import { lookup, noop } from './util'
import { runStats } from './util/runStats'

export const mkSubscriptionSet = (source: Source, subscriptions?: ReadonlyArray<Subscription>) => {
  // filter out subscriptions to jelly fish, which are not equal to source
  // jelly events are considered ephemeral, that's why we prohibit subscribing to them
  const subscriptions0 =
    subscriptions &&
    subscriptions.filter(s0 => !Semantics.isJelly(s0.semantics) || R.equals(source, s0))
  // add a self-subscription in case the list is empty
  const subscriptions1 = subscriptions0 && subscriptions0.length > 0 ? subscriptions0 : [source]
  return SubscriptionSet.or(subscriptions1)
}

export type FishJar<C, E, P> = Readonly<{
  // enqueue the commands for processing
  enqueueCommand: (command: C, onComplete: () => void, onError: (err: any) => void) => void

  // public "state"
  publicSubject: Observable<P>

  dispose: () => void

  dump: () => string
}>

type CommandInput<C> = Readonly<{
  type: 'command'
  command: C
  onComplete: () => void
  onError: (err: any) => void
}>

type EventScanState<S, E> = Readonly<{
  eventStore: FishEventStore<S, E>
  /**
   * Sometimes we do not have to emit a new state. E.g. when a command
   * does not result in events.
   */
  emit: ReadonlyArray<StateWithProvenance<S>>
}>

type CommandScanState = Readonly<{
  waitFor: Psn
}>

const createFishJar = <S, C, E, P>(
  source: Source,
  subscriptions: SubscriptionSet,
  fish: FishTypeImpl<S, C, E, P>,
  store: FishEventStore<S, E>,
  storeState: StateWithProvenance<S>,
  observe: ObserveMethod, // observe other fishes
  sendToStore: SendToStore, // send chunk to store
  realtimeEvents: (
    filter: SubscriptionSet,
    from: OffsetMap,
  ) => Observable<ReadonlyArray<EnvelopeFromStore<E>>>, // get realtime events
  commandExecutor: CommandExecutor, // for async effects
  offsetMap: OffsetMap,
  pondStateTracker: PondStateTracker,
): FishJar<C, E, P> => {
  // does this even have to be a subject?
  const eventsIn = realtimeEvents(subscriptions, offsetMap)
  const commandIn: Subject<CommandInput<C>> = new Subject()
  const publicSubject: Subject<P> = new ReplaySubject<P>(1)
  const stateSubject = new ReplaySubject<StateWithProvenance<S>>(1)
  const privateStateOut = stateSubject.map(sp => sp.state)
  const pondObservables: PondObservables<S> = {
    observe,
    observeSelf: () => privateStateOut,
  }

  const eventFilter = subscriptionsToEnvelopePredicate(subscriptions)

  // Initial state for the mergeScan pipeline - this is the state of the Jar right after replay from FishEventStore
  const mergeScanSeed: EventScanState<S, E> = {
    eventStore: store,
    emit: [storeState],
  }

  const enqueueSelfCommand = (command: C): void =>
    commandIn.next({
      type: 'command',
      command,
      onComplete: noop,
      onError: noop,
    })

  // this does not return an observable, because it is fully synchronous and nothing can fail.
  const runStateEffect = (effect: StateEffect<C, P>): ReadonlyArray<P> => {
    switch (effect.type) {
      case 'sendSelfCommand': {
        enqueueSelfCommand(effect.command)
        return []
      }
      case 'publish': {
        return [effect.state]
      }
    }
  }

  // Aggregate incoming events into ever-new states.
  // We reveal the Provenance too, so that downstream consumers can implement specialized logic.
  const evScanAcc = (
    current: EventScanState<S, E>,
    events: ReadonlyArray<EnvelopeFromStore<E>>,
  ): Observable<EventScanState<S, E>> => {
    const start = Timestamp.now()
    const pondStateTrackerEventProcessingToken = pondStateTracker.eventsFromOtherSourcesProcessingStarted(
      source.semantics,
      source.name,
    )

    const unblock = () =>
      pondStateTracker.eventsFromOtherSourcesProcessingFinished(
        pondStateTrackerEventProcessingToken,
      )

    try {
      const profile = `inject-events/${fish.semantics}`

      runStats.durations.start(profile, start)
      const needsState = current.eventStore.processEvents(events)
      runStats.durations.end(profile, start, Timestamp.now())

      const result = needsState
        ? current.eventStore
            .currentState()
            .pipe(runStats.profile.profileObservable(`inject-compute/${fish.semantics}`))
            .map(s => ({
              ...current,
              emit: [s],
            }))
        : Observable.of({ ...current, emit: [] })

      return result.pipe(
        tap(
          unblock,
          // On errors, also update the tracker
          unblock,
        ),
      )
    } catch (e) {
      // Synchronous error, for example from onEvent
      unblock()
      throw e
    }
  }

  // executes effects and returns a promise of the produced events
  const handleAsyncCommandResult = (command: C) => (ar: CommandApi<ReadonlyArray<E>>) =>
    Observable.fromPromise(commandExecutor(ar))
      .concatMap(events => {
        // do not process events if undefined, log only
        if (events === undefined) {
          log.pond.error('undefined commandResult', command, 'ar', ar)
          // TODO: distress
          return Observable.of([])
        } else {
          return sendToStore(source, events)
        }
      })
      .catch(e => {
        // TODO: distress?
        log.pond.error(e)
        return Observable.of([])
      })

  // Command handling pipeline. After each command, if it emitted any events that we are subscribed to,
  // the handling of the following command is delayed until upstream (event aggregation) has seen and
  // integrated the event into our state.
  // In this way, we arrive at our core command guarantee: Every command sees all local effects of all
  // preceding commands.
  const cmdScanAcc = (
    current: CommandScanState,
    input: CommandInput<C>,
  ): Observable<CommandScanState> => {
    const { command, onComplete, onError } = input
    const pondStateTrackerCommandProcessingToken = pondStateTracker.commandProcessingStarted(
      source.semantics,
      source.name,
    )
    const unblock = () =>
      pondStateTracker.commandProcessingFinished(pondStateTrackerCommandProcessingToken)

    const result = stateSubject
      .filter(stateWithProvenance => {
        if (current.waitFor < Psn.zero) {
          return true
        }

        const latestSeen = lookup(stateWithProvenance.psnMap, source.sourceId)
        const pass = latestSeen !== undefined && latestSeen >= current.waitFor

        if (!pass) {
          log.pond.debug(
            Source.format(source),
            'waiting for',
            current.waitFor,
            '; currently at:',
            latestSeen,
          )
        }
        return pass
      })
      .map(sp => sp.state)
      .take(1)
      .concatMap(s => {
        const onCommandResult = fish.onCommand(s, command)
        const stored = CommandResult.fold<E, Observable<ReadonlyArray<EnvelopeFromStore<E>>>>(
          onCommandResult,
        )({
          sync: events => sendToStore(source, events),
          async: handleAsyncCommandResult(command),
          none: () => Observable.of([]),
        })

        return stored.concatMap(envelopes => {
          if (envelopes.length === 0) {
            return Observable.of({ ...current })
          }

          // We only care about events we ourselves are actually subscribed to.
          const filtered = envelopes.filter(eventFilter)
          if (filtered.length === 0) {
            return Observable.of({ ...current })
          }

          // We must wait for the final psn of our generated events
          // to be applied to the state, before we may apply the next command.
          const finalPsn = filtered[filtered.length - 1].psn

          return Observable.of({ waitFor: finalPsn })
        })
      })

    return result.pipe(
      catchError(x => {
        unblock()
        onError(x)
        return Observable.of(current)
      }),
      tap(() => {
        unblock()
        onComplete()
      }),
    )
  }

  const rxSubs: RxSubscription[] = []

  // Must subscribe to events BEFORE starting OnStateChange pipeline,
  // since in OnStateChange we may immediately trigger effects in OTHER fish,
  // which we must see via the realtime stream.
  rxSubs.push(
    Observable.concat(
      Observable.of(mergeScanSeed),
      eventsIn.filter(evs => evs.length > 0).mergeScan(evScanAcc, mergeScanSeed, 1),
    )
      .concatMap(x => x.emit)
      .subscribeOn(Scheduler.queue)
      .subscribe(stateSubject),
  )

  // Must wire the commandIn topic before starting the onStateChange pipeline,
  // since it may immediately emit self-commands.
  rxSubs.push(commandIn.mergeScan(cmdScanAcc, { waitFor: Psn.min }, 1).subscribe())

  rxSubs.push(
    fish
      .onStateChange(pondObservables)
      .subscribeOn(Scheduler.queue)
      .concatMap(runStateEffect)
      .subscribe(publicSubject),
  )

  const dispose = (): void => {
    rxSubs.forEach(sub => sub.unsubscribe())
  }

  // enqueue the commands for processing
  const enqueueCommand = (
    command: C,
    onComplete: () => void,
    onError: (err: any) => void,
  ): void => {
    commandIn.next({
      type: 'command',
      command,
      onComplete,
      onError,
    })
  }

  const dump = (): string => {
    // this relies on the store being mutable, otherwise we would always get the initial store
    const events = store.currentEvents()
    const eventsLen = events.length
    // const hash = sha1Hash(events) too expensive for large setups!
    return `${source.semantics}/${source.name} ${eventsLen}`
  }

  return {
    dispose,
    enqueueCommand,
    dump,
    publicSubject,
  }
}
export const createSubscriptionLessFishJar = <S, C, E, P>(
  fish: FishTypeImpl<S, C, E, P>,
  initialState: S,
  source: Source,
  observe: <C1, E1, P1>(fish: FishType<C1, E1, P1>, fishName: string) => Observable<P1>,
  sendEventChunk: SendToStore,
  commandExecutor: CommandExecutor,
): Observable<FishJar<C, E, P>> => {
  const enqueueCommand = (command: C, onComplete: () => void, onError: (err: any) => void) => {
    const commandResult = fish.onCommand(initialState, command)
    CommandResult.fold(commandResult)({
      async: cr => {
        commandExecutor(cr)
          .then(events => {
            sendEventChunk(source, events).subscribe()
          })
          .then(onComplete)
          .catch(e => {
            // TODO: distress?
            log.pond.error(e)
            onError(e)
            return Observable.of([])
          })
      },
      sync: events => {
        sendEventChunk(source, events)
          .do(onComplete)
          .catch(e => {
            // TODO: distress?
            log.pond.error(e)
            onError(e)
            return Observable.of([])
          })
          .subscribe()
      },
      none: noop,
    })
  }

  const runStateEffect = (effect: StateEffect<C, P>): ReadonlyArray<P> => {
    switch (effect.type) {
      case 'sendSelfCommand': {
        enqueueCommand(effect.command, noop, noop)
        return []
      }
      case 'publish': {
        return [effect.state]
      }
    }
  }
  return Observable.of({
    dispose: noop,
    dump: () => `${source.semantics}/${source.name} 0`,
    publicSubject: fish
      .onStateChange({ observe, observeSelf: () => Observable.of(initialState) })
      .observeOn(Scheduler.queue)
      .concatMap(runStateEffect),
    enqueueCommand,
  })
}

export const hydrate = <S, C, E, P>(
  fish: FishTypeImpl<S, C, E, P>,
  fishName: FishName,
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  sendEventChunk: SendToStore,
  observe: <C1, E1, P1>(fish: FishType<C1, E1, P1>, fishName: string) => Observable<P1>,
  commandExecutor: CommandExecutor,
  pondStateTracker: PondStateTracker,
): Observable<FishJar<C, E, P>> => {
  log.pond.debug('hydrating', fish.semantics, `"${fishName}"`)

  const stats = runStats.profile.profileObservable
  const startMs = Date.now()

  const { sourceId } = eventStore
  const source: Source = { semantics: fish.semantics, name: fishName, sourceId }
  const { state: initialState, subscriptions } = fish.initialState(source.name, source.sourceId)
  if (
    !Semantics.isJelly(fish.semantics) &&
    subscriptions !== undefined &&
    subscriptions.length === 0
  ) {
    return createSubscriptionLessFishJar(
      fish,
      initialState,
      source,
      observe,
      sendEventChunk,
      commandExecutor,
    )
  }

  // Jelly and normal subscriptions cannot be mixed anymore after the switch to lamport order,
  // since we are assignign the jelly lamports locally in the pond currently.
  // Cf. https://github.com/Actyx/Cosmos/issues/2797
  if (subscriptions && subscriptions.some(sub => Semantics.isJelly(sub.semantics))) {
    if (subscriptions.every(sub => Semantics.isJelly(sub.semantics))) {
      // We do prefer the implicit self-only subscription of a jelly-fish...
      log.pond.warn(
        fishName,
        'requested explicit jelly fish subscriptions. We will allow it since they are ALL jelly.',
      )
    } else {
      throw new Error(
        'Mixing jelly and normal subscriptions is not allowed, found some for: ' + fishName,
      )
    }
  }

  const token = pondStateTracker.hydrationStarted(fish.semantics, fishName)
  const subscriptionSet = mkSubscriptionSet(source, subscriptions)

  const realtimeEvents = (
    filter: SubscriptionSet,
    from: OffsetMap,
  ): Observable<ReadonlyArray<EnvelopeFromStore<any>>> =>
    eventStore
      .allEvents(
        {
          psns: from,
          default: 'min',
        },
        { psns: {}, default: 'max' },
        filter,
        AllEventsSortOrders.Unsorted,
        // EventKey.zero, // optional
      )
      .concatMap(intoOrderedChunks)
      .map(x => x.map(ev => Event.toEnvelopeFromStore<E>(ev)))

  const present = eventStore.present()

  const info: FishInfo<S, E> = {
    semantics: fish.semantics,
    fishName,
    initialState,
    subscriptionSet,
    onEvent: fish.onEvent,
    isSemanticSnapshot: fish.semanticSnapshot
      ? fish.semanticSnapshot(fishName, sourceId)
      : undefined,
    snapshotFormat: fish.localSnapshot,
  }
  const snapshotScheduler = SnapshotScheduler.create(10)
  return Observable.zip(
    present,
    present
      .take(1)
      .concatMap(p =>
        FishEventStore.initialize(info, eventStore, snapshotStore, snapshotScheduler, p.psns).pipe(
          stats(`initial-getevents/${fish.semantics}`),
        ),
      ),
  ).concatMap(([psnMap, store]) => {
    return store
      .currentState()
      .pipe(stats(`initial-compute/${fish.semantics}`))
      .do(() => pondStateTracker.hydrationFinished(token))
      .map(storeState =>
        createFishJar(
          source,
          subscriptionSet,
          fish,
          store,
          storeState,
          observe,
          sendEventChunk,
          realtimeEvents,
          commandExecutor,
          psnMap.psns,
          pondStateTracker,
        ),
      )
      .do(jar => {
        log.pond.debug(
          'finished initializing fish jar',
          fish.semantics,
          `"${fishName}"`,
          'in',
          (Date.now() - startMs) / 1000,
          'seconds from ',
          jar.dump(),
        )
      })
  })
}

export const FishJar = {
  hydrate,
}
