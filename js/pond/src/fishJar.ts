/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as R from 'ramda'
import { clone } from 'ramda'
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
import { AllEventsSortOrders, Event, Events, OffsetMap } from './eventstore/types'
import { intoOrderedChunks } from './eventstore/utils'
import { CommandExecutor } from './executors/commandExecutor'
import { FishEventStore, FishInfo } from './fishEventStore'
import log from './loggers'
import { SendToStore } from './pond'
import { PondStateTracker } from './pond-state'
import { EntityId, Metadata } from './pond-v2-types'
import { SnapshotStore } from './snapshotStore'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { Subscription, SubscriptionSet, subscriptionsToEventPredicate } from './subscription'
import {
  AsyncCommandResult,
  ObserveMethod,
  Psn,
  Semantics,
  SnapshotFormat,
  SourceId,
  StateWithProvenance,
  SyncCommandResult,
  Timestamp,
} from './types'
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

// I is an intermediate value that is consumed by the specialized command handling logic.
// Pond V1 has Async vs. SyncCommandResult, while V2 has Payload+Tags.
export type CommandFn<S, I> = (state: S) => I

export type FishJar<C, E, P> = Readonly<{
  // enqueue the commands for processing
  enqueueCommand: (command: C, onComplete: () => void, onError: (err: any) => void) => void

  // public "state"
  publicSubject: Observable<P>

  dispose: () => void

  dump: () => string
}>

type CommandInput<S, I> = Readonly<{
  type: 'command'
  command: CommandFn<S, I>
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

const mkEventScanAcc = <S, E>(
  pondStateTracker: PondStateTracker,
  semantics: Semantics,
  name: FishName,
) => {
  // Aggregate incoming events into ever-new states.
  // We reveal the Provenance too, so that downstream consumers can implement specialized logic.
  const evScanAcc = (
    current: EventScanState<S, E>,
    events: Events,
  ): Observable<EventScanState<S, E>> => {
    const start = Timestamp.now()
    const pondStateTrackerEventProcessingToken = pondStateTracker.eventsFromOtherSourcesProcessingStarted(
      semantics,
      name,
    )

    const unblock = () =>
      pondStateTracker.eventsFromOtherSourcesProcessingFinished(
        pondStateTrackerEventProcessingToken,
      )

    try {
      const profile = `inject-events/${semantics}`

      runStats.durations.start(profile, start)
      const needsState = current.eventStore.processEvents(events)
      runStats.durations.end(profile, start, Timestamp.now())

      const result = needsState
        ? current.eventStore
            .currentState()
            .pipe(runStats.profile.profileObservable(`inject-compute/${semantics}`))
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

  return evScanAcc
}

export type CommandPipeline<S, I> = Readonly<{
  // Subject where new commands must be pushed
  subject: Subject<CommandInput<S, I>>

  // Subscription to the running pipeline (cancel to destroy pipeline)
  subscription: RxSubscription
}>

type CommandScanState = Readonly<{
  waitFor: Psn
}>

const commandPipeline = <S, I>(
  pondStateTracker: PondStateTracker,
  localSourceId: SourceId,
  semantics: string,
  name: string,
  handler: ((input: I) => Observable<Events>),
  stateSubject: Observable<StateWithProvenance<S>>,
  eventFilter: ((t: Event) => boolean),
): CommandPipeline<S, I> => {
  const commandIn: Subject<CommandInput<S, I>> = new Subject()

  // Command handling pipeline. After each command, if it emitted any events that we are subscribed to,
  // the handling of the following command is delayed until upstream (event aggregation) has seen and
  // integrated the event into our state.
  // In this way, we arrive at our core command guarantee: Every command sees all local effects of all
  // preceding commands.
  const cmdScanAcc = (
    current: CommandScanState,
    input: CommandInput<S, I>,
  ): Observable<CommandScanState> => {
    const { command, onComplete, onError } = input

    const pondStateTrackerCommandProcessingToken = pondStateTracker.commandProcessingStarted(
      semantics,
      name,
    )
    const unblock = () => {
      pondStateTracker.commandProcessingFinished(pondStateTrackerCommandProcessingToken)
    }

    const result = stateSubject
      .filter(stateWithProvenance => {
        if (current.waitFor < Psn.zero) {
          return true
        }

        const latestSeen = lookup(stateWithProvenance.psnMap, localSourceId)
        const pass = latestSeen !== undefined && latestSeen >= current.waitFor

        if (!pass) {
          log.pond.debug(
            semantics,
            '/',
            name,
            '/',
            localSourceId,
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
        const onCommandResult = command(s)
        const stored = handler(onCommandResult)

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

  const subscription = commandIn.mergeScan(cmdScanAcc, { waitFor: Psn.min }, 1).subscribe()

  return {
    subject: commandIn,
    subscription,
  }
}

const createFishJar = <S, C, E, P>(
  source: Source,
  subscriptions: SubscriptionSet,
  fish: FishTypeImpl<S, C, E, P>,
  store: FishEventStore<S, E>,
  storeState: StateWithProvenance<S>,
  observe: ObserveMethod, // observe other fishes
  sendToStore: SendToStore, // send chunk to store
  realtimeEvents: (filter: SubscriptionSet, from: OffsetMap) => Observable<Events>, // get realtime events
  commandExecutor: CommandExecutor, // for async effects
  offsetMap: OffsetMap,
  pondStateTracker: PondStateTracker,
): FishJar<C, E, P> => {
  type LegacyCmdRes = SyncCommandResult<E> | AsyncCommandResult<E>

  // does this even have to be a subject?
  const eventsIn = realtimeEvents(subscriptions, offsetMap)

  const publicSubject: Subject<P> = new ReplaySubject<P>(1)
  const stateSubject = new ReplaySubject<StateWithProvenance<S>>(1)
  const privateStateOut = stateSubject.map(sp => sp.state)
  const pondObservables: PondObservables<S> = {
    observe,
    observeSelf: () => privateStateOut,
  }

  const eventFilter = subscriptionsToEventPredicate(subscriptions)

  // Initial state for the mergeScan pipeline - this is the state of the Jar right after replay from FishEventStore
  const mergeScanSeed: EventScanState<S, E> = {
    eventStore: store,
    emit: [storeState],
  }

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
  const evScanAcc = mkEventScanAcc<S, E>(pondStateTracker, source.semantics, source.name)

  // executes effects and returns a promise of the produced events
  const handleAsyncCommandResult = (ar: CommandApi<ReadonlyArray<E>>) =>
    Observable.fromPromise(commandExecutor(ar))
      .concatMap(events => {
        // do not process events if undefined, log only
        if (events === undefined) {
          log.pond.error('undefined commandResult', '<command omitted>', 'ar', ar)
          // TODO: distress
          return Observable.of([])
        } else {
          return sendToStore(source.semantics, source.name, [], events)
        }
      })
      .catch(e => {
        // TODO: distress?
        log.pond.error(e)
        return Observable.of([])
      })

  const handleCommandResult = (onCommandResult: LegacyCmdRes) =>
    CommandResult.fold<E, Observable<Events>>(onCommandResult)({
      sync: events => sendToStore(source.semantics, source.name, [], events),
      async: handleAsyncCommandResult,
      none: () => Observable.of([]),
    })

  const cmdPipeline = commandPipeline<S, LegacyCmdRes>(
    pondStateTracker,
    source.sourceId,
    source.semantics,
    source.name,
    handleCommandResult,
    stateSubject,
    eventFilter,
  )

  const enqueueSelfCommand = (command: C): void =>
    cmdPipeline.subject.next({
      type: 'command',
      command: (s: S) => fish.onCommand(s, command),
      onComplete: noop,
      onError: noop,
    })

  // Must wire the commandIn topic before starting the onStateChange pipeline,
  // since it may immediately emit self-commands.
  const rxSubs: RxSubscription[] = [cmdPipeline.subscription]

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
    cmdPipeline.subject.next({
      type: 'command',
      command: (s: S) => fish.onCommand(s, command),
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
            sendEventChunk(source.semantics, source.name, [], events).subscribe()
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
        sendEventChunk(source.semantics, source.name, [], events)
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

  const realtimeEvents = (filter: SubscriptionSet, from: OffsetMap): Observable<Events> =>
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

  const present = eventStore.present()

  const info: FishInfo<S, E> = {
    semantics: fish.semantics,
    fishName,
    initialState: () => fish.initialState(source.name, source.sourceId).state,
    subscriptionSet,
    onEvent: (state, ev) => fish.onEvent(state, Event.toEnvelopeFromStore<E>(ev)),
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

const hydrateV2 = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  pondStateTracker: PondStateTracker,
) => <S, E>(
  subscriptionSet: SubscriptionSet,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  cacheKey: EntityId,
  enableLocalSnapshots: boolean,
  isReset?: (event: E) => boolean,
): Observable<StateWithProvenance<S>> => {
  const snapshotScheduler = SnapshotScheduler.create(10)
  const semantics = cacheKey.entityType
    ? Semantics.of(cacheKey.entityType)
    : Semantics.internal('untyped-aggregation')
  const fishName = FishName.of(cacheKey.name)

  const { sourceId } = eventStore

  // We construct a "Fish" from the given parameters in order to use the unchanged FES.
  const info: FishInfo<S, E> = {
    semantics,
    fishName,
    initialState: () => clone(initialState),
    subscriptionSet,

    onEvent: (state, ev) =>
      onEvent(state, ev.payload as E, {
        isLocalEvent: ev.sourceId === sourceId,
        tags: ev.tags,
        timestampMicros: ev.timestamp,
        timestampAsDate: Timestamp.toDate.bind(null, ev.timestamp),
        lamport: ev.lamport,
      }),

    isSemanticSnapshot: isReset ? envelope => isReset(envelope.payload) : undefined,

    // TODO proper support
    snapshotFormat: enableLocalSnapshots
      ? SnapshotFormat.identity(cacheKey.version || 0)
      : undefined,
  }

  return eventStore
    .present()
    .take(1)
    .concatMap(present => {
      const init = FishEventStore.initialize(
        info,
        eventStore,
        snapshotStore,
        snapshotScheduler,
        present.psns,
      )

      return init.map(fes => ({ fes, present }))
    })
    .concatMap(({ present, fes }) => {
      const liveEvents = eventStore
        .allEvents(
          {
            psns: present.psns,
            default: 'min',
          },
          { psns: {}, default: 'max' },
          subscriptionSet,
          AllEventsSortOrders.Unsorted,
          // EventKey.zero, // optional
        )
        .concatMap(intoOrderedChunks)
        .filter(evs => evs.length > 0)

      const mergeScanSeed: EventScanState<S, E> = {
        eventStore: fes,
        emit: [],
      }

      const accumulator = mkEventScanAcc<S, E>(pondStateTracker, semantics, fishName)

      return Observable.concat(
        fes.currentState().take(1),
        liveEvents.mergeScan(accumulator, mergeScanSeed, 1).concatMap(x => x.emit),
      )
    })
}

export const FishJar = {
  hydrate,
  hydrateV2,
  commandPipeline,
}
