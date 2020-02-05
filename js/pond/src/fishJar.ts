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
import { ObserveMethod, Semantics, Timestamp } from './types'
import { noop } from './util'
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

type EventInput<E> = Readonly<{
  type: 'events'
  events: ReadonlyArray<EnvelopeFromStore<E>>
}>
type CommandInput<C> = Readonly<{
  type: 'command'
  command: C
  onComplete: () => void
  onError: (err: any) => void
}>
type ScanInput<E, C> = EventInput<E> | CommandInput<C>

type MergeScanState<S, E> = Readonly<{
  eventStore: FishEventStore<S, E>
  /**
   * Sometimes we do not have to emit a new state. E.g. when a command
   * does not result in events.
   */
  emit: ReadonlyArray<S>
  // this can be used to async get current state
  // contains all events from the beginning of time for this fish (SIGH!)
  // also contains the initial state
}>

const createFishJar = <S, C, E, P>(
  source: Source,
  subscriptions: SubscriptionSet,
  fish: FishTypeImpl<S, C, E, P>,
  store: FishEventStore<S, E>,
  storeState: S,
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
  const eventsIn: Subject<ReadonlyArray<EnvelopeFromStore<E>>> = new Subject()
  const commandIn: Subject<CommandInput<C>> = new Subject()
  const publicSubject: Subject<P> = new ReplaySubject<P>(1)
  const stateSubject = new ReplaySubject<S>(1)
  const pondObservables: PondObservables<S> = {
    observe,
    observeSelf: () => stateSubject,
  }
  const in1: Observable<ScanInput<E, C>> = eventsIn
    // must not filter out own events (even though we’ll apply them directly) because other Ponds on the
    // same store may also contribute events here, with the same sourceId
    .map<ReadonlyArray<EnvelopeFromStore<E>>, ScanInput<E, C>>(events => ({
      type: 'events',
      events,
    }))
  const in2: Observable<ScanInput<E, C>> = commandIn
  const mergeScanInput: Observable<ScanInput<E, C>> = Observable.merge(in1, in2)

  const eventFilter = subscriptionsToEnvelopePredicate(subscriptions)

  // Initial state for the mergeScan pipeline - this is the state of the Jar right after replay from FishEventStore
  const mergeScanSeed: MergeScanState<S, E> = {
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

  const processEvents = (events: ReadonlyArray<EnvelopeFromStore<E>>): void => eventsIn.next(events)

  // Accumulator function for the mergeScanPipeline, our Master Control Program
  const mergeScanAccumulator = (
    current: MergeScanState<S, E>,
    input: ScanInput<E, C>,
  ): Observable<MergeScanState<S, E>> => {
    switch (input.type) {
      case 'command': {
        const { command, onComplete, onError } = input
        const pondStateTrackerCommandProcessingToken = pondStateTracker.commandProcessingStarted(
          source.semantics,
          source.name,
        )
        const unblock = () =>
          pondStateTracker.commandProcessingFinished(pondStateTrackerCommandProcessingToken)

        const result = current.eventStore.currentState().concatMap(s => {
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
              return Observable.of({ ...current, emit: [] })
            }

            const filtered = envelopes.filter(eventFilter)
            if (filtered.length === 0) {
              return Observable.of({ ...current, emit: [] })
            }

            const start = Timestamp.now()
            const profile = `command-events/${fish.semantics}`

            runStats.durations.start(profile, start)

            // Here we put our own events into the store, we’ll get them again as live events!
            // This mechanism is necessary so that we can guarantee to wait with processing the
            // next command until this command has had its proper effect on the fish state.
            const needsState = current.eventStore.processEvents(filtered)
            runStats.durations.end(profile, start, Timestamp.now())

            return needsState
              ? current.eventStore
                  .currentState()
                  .pipe(runStats.profile.profileObservable(`command-compute/${fish.semantics}`))
                  .map(state1 => ({
                    eventStore: current.eventStore,
                    emit: [state1],
                  }))
              : Observable.of({ ...current, emit: [] })
          })
        })

        return result.pipe(
          catchError(x => {
            unblock()
            onError(x)
            return Observable.throw(x)
          }),
          tap(() => {
            unblock()
            onComplete()
          }),
        )
      }

      case 'events': {
        // we assume that we are not getting our own events here!
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
          const needsState = current.eventStore.processEvents(input.events)
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
    }
  }
  // we need to explicitly emit the mergeScanSeed, since the mergeScan will not emit
  // the seed for a non-terminating stream:
  //
  // Rx.Observable.never().mergeScan(x => x, 'seed', 1).do(x => console.log(x)).subscribe() // never prints seed
  //
  // however, for a terminating stream the seed is emitted:
  //
  // Rx.Observable.empty().mergeScan(x => x, 'seed', 1).do(x => console.log(x)).subscribe() // does print seed (on complete, I guess)
  //
  // Here and in some other places (notably fishEventStore2.ts, pond.ts and unsubscribeOn.ts) you will see reference to Scheduler.queue
  // Its purpose is avoidance of stack overflows in the rxjs pipelines, especially ones containing sources emitting large numbers
  // of elements. The root cause of those stack overflows is the move of rxjs to more aggresive behaviour with respect to observable emissions
  // with version change from 4 to 5. This was supposed to be some 'performance optimisation'. It results (especially for inner observables
  // in concatMap) in eager generation of code thunks for the whole pipeline for each new upcoming element even when the previous ones have
  // not been consumed, finally causing the pipeline to exceed the stack limit. The recommended workaround is to use a non-immediate Scheduler,
  // like Scheduler.queue (previously known as 'trampoline scheduler') that puts an intermittent queue which causes the thunks to be produced
  // only for the elements that are being de-queued.
  // unsubscribeOn is a bit more involved example, which battles stack-overflow on eager unsubscription, which without non-immediate Scheduler
  // would result in generating code for unsubscription from each generated element.

  const rxSubs: RxSubscription[] = []
  rxSubs.push(
    Observable.concat(
      Observable.of(mergeScanSeed),
      mergeScanInput.mergeScan(mergeScanAccumulator, mergeScanSeed, 1),
    )
      .concatMap(x => x.emit)
      .subscribe(stateSubject),
  )

  rxSubs.push(
    fish
      .onStateChange(pondObservables)
      .observeOn(Scheduler.queue)
      .concatMap(runStateEffect)
      .subscribe(publicSubject),
  )

  rxSubs.push(
    // this includes all our subscriptions, also our own emitted events
    realtimeEvents(subscriptions, offsetMap)
      .do(processEvents)
      .subscribe(),
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
        pondStateTracker.hydrationFinished(token)
        log.pond.debug(
          'finished hydrating',
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
