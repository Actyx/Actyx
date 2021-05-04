/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  CancelSubscription,
  Metadata,
  Milliseconds,
  NodeId,
  PendingEmission,
  Tags,
  Timestamp,
  toEventPredicate,
  Where,
} from '@actyx/sdk'
import { Observable, ReplaySubject, Scheduler, Subject, Subscription } from 'rxjs'
import { CommandInterface } from './commandInterface'
import { EventStore, UnstoredEvent, WsStoreConfig } from './eventstore'
import { MultiplexedWebsocket } from './eventstore/multiplexedWebsocket'
import { TestEvent } from './eventstore/testEventStore'
import { AllEventsSortOrders, ConnectivityStatus, Events } from './eventstore/types'
import { extendDefaultWsStoreConfig, mkMultiplexer } from './eventstore/utils'
import { getNodeId } from './eventstore/websocketEventStore'
import { CommandPipeline, FishJar, StartedFishMap } from './fishJar'
import log from './loggers'
import { observeMonotonic } from './monotonic'
import { mkPondStateTracker, PondState, PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { SplashState, streamSplashState, WaitForSwarmConfig } from './splashState'
import { Monitoring } from './store/monitoring'
import { SnapshotScheduler } from './store/snapshotScheduler'
import {
  AddEmission,
  Caching,
  Fish,
  FishErrorReporter,
  FishId,
  IsReset,
  ObserveAllOpts,
  Reduce,
  StateEffect,
  StateWithProvenance,
} from './types'
import { noop } from './util'

const isTyped = (e: ReadonlyArray<string> | Tags<unknown>): e is Tags<unknown> => {
  return !Array.isArray(e)
}

/** Advanced configuration options for the Pond. @public */
export type PondOptions = {
  hbHistDelay?: number
  currentPsnHistoryDelay?: number
  updateConnectivityEvery?: Milliseconds

  stateEffectDebounce?: number

  fishErrorReporter?: FishErrorReporter
}

/** Information concerning the running Pond. @public */
export type PondInfo = {
  nodeId: NodeId
}

const omitObservable = <S>(
  stoppedByError: ((err: unknown) => void) | undefined,
  callback: (newState: S) => void,
  states: Observable<StateWithProvenance<S>>,
): CancelSubscription => {
  try {
    // Not passing an error callback seems to cause bad behavior with RXjs internally
    const sub = states
      .map(x => x.state)
      .subscribe(callback, typeof stoppedByError === 'function' ? stoppedByError : noop)
    return sub.unsubscribe.bind(sub)
  } catch (err) {
    stoppedByError && stoppedByError(err)
    return noop
  }
}

const wrapStateFn = <S, EWrite>(fn: StateEffect<S, EWrite>) => {
  const effect = async (state: S) => {
    const emissions: Emit<any>[] = []
    let returned = false

    const enqueueEmission: AddEmission<EWrite> = (tags, payload) => {
      if (returned) {
        throw new Error(
          'The function you passed to run/keepRunning has already returned -- enqueuing emissions via the passed "AddEmission" function is no longer possible.',
        )
      }

      emissions.push({ tags, payload })
    }

    await fn(state, enqueueEmission)
    returned = true

    return emissions
  }

  return effect
}

const pendingEmission = (o: Observable<void>): PendingEmission => ({
  subscribe: o.subscribe.bind(o),
  toPromise: () => o.toPromise(),
})

// For internal use only. TODO: Cleanup.
type Emit<E> = {
  tags: ReadonlyArray<string> | Tags<E>
  payload: E
}
type StateEffectInternal<S, EWrite> = (state: S) => EmissionRequest<EWrite>
type EmissionRequest<E> = ReadonlyArray<Emit<E>> | Promise<ReadonlyArray<Emit<E>>>
// endof TODO cleanup

type ActiveFish<S> = {
  readonly states: Subject<StateWithProvenance<S>>
  readonly subscription: Subscription
  commandPipeline?: CommandPipeline<S, EmissionRequest<any>>
}

/** Parameter object for the `Pond.getNodeConnectivity` call. @public */
export type GetNodeConnectivityParams = Readonly<{
  callback: (newState: ConnectivityStatus) => void
  specialSources?: ReadonlyArray<NodeId>
}>

/** Parameter object for the `Pond.waitForSwarmSync` call. @public */
export type WaitForSwarmSyncParams = WaitForSwarmConfig &
  Readonly<{
    onSyncComplete: () => void
    onProgress?: (newState: SplashState) => void
  }>

/**
 * Main interface for interaction with the ActyxOS event system.
 * New instances are created via `Pond.default()` or `Pond.of(options)`.
 * Acquire a Pond for testing (which uses a simulated clean Event Store) via `Pond.test()`.
 * @public
 */
export type Pond = {
  /* EMISSION */

  /**
   * Emit a single event directly.
   *
   * @typeParam E  - Type of the event payload. If your tags are statically declared,
   *                 their type will be checked against the payload’s type.
   *
   * @param tags   - Tags to attach to the event. E.g. `Tags('myTag', 'myOtherTag')`
   * @param event  - The event itself.
   * @returns        A `PendingEmission` object that can be used to register
   *                 callbacks with the emission’s completion.
   */
  emit<E>(tags: Tags<E>, event: E): PendingEmission

  /* AGGREGATION */

  /**
   * Observe the current state of a Fish.
   *
   * Caching is done based on the `fishId` inside the `fish`, i.e. if a fish with the included
   * `fishId` is already known, that other Fish’s ongoing aggregation will be used instead of
   * starting a new one.
   *
   * @param fish       - Complete Fish information.
   * @param callback   - Function that will be called whenever a new state becomes available.
   * @param stoppedByError - Function that will be called when one of the Fish’s functions throws an error.
   *                         A Fish will always stop emitting further states after errors, even if no `stoppedByError` argument is passed.
   * @returns            A function that can be called in order to cancel the subscription.
   */
  observe<S, E>(
    fish: Fish<S, E>,
    callback: (newState: S) => void,
    stoppedByError?: (err: unknown) => void,
  ): CancelSubscription

  /**
   * Create Fish from events and observe them all.
   * Note that if a Fish created from some event f0 will also observe events earlier than f0, if they are selected by `where`
   *
   * @typeParam F        - Type of the events used to initialize Fish.
   * @typeParam S        - Type of the observed Fish’s state.
   *
   * @param seedEventsSelector  - A `Where<F>` object identifying the seed events to start Fish from
   * @param makeFish     - Factory function to create a Fish with state `S` from an event of type `F`.
   *                       If Fish with same FishId are created by makeFish, these Fish must be identical!
   *                       `undefined` may be returned to indicate the given seed event should not be converted to a Fish at all.
   * @param opts         - Optional arguments regarding caching and expiry
   * @param callback     - Function that will be called with the array of states whenever the set of Fish
   *                       changes or any of the contained Fish’s state changes.
   *
   * @returns              A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeAll<ESeed, S>(
    // Expression to extract the initial events, e.g. Tag<TaskCreated>
    seedEventsSelector: Where<ESeed>,
    // Create a concrete Fish from the initial event
    makeFish: (seedEvent: ESeed) => Fish<S, any> | undefined,
    // When to remove Fish from the "all" set.
    opts: ObserveAllOpts,
    callback: (states: S[]) => void,
  ): CancelSubscription

  /**
   * Find the event selected by `firstEvent`, and start a Fish from it.
   * It is legal for `firstEvent` to actually select multiple events;
   * however, `makeFish` must yield the same Fish no matter one is passed in.
   *
   * @typeParam F        - Type of the initial event.
   * @typeParam S        - Type of the observed Fish’s state.
   *
   * @param seedEventSelector   - A `Where<F>` object identifying the seed event
   * @param makeFish     - Factory function to create the Fish with state `S` from the event of type `F`.
   *                       The Fish is able to observe events earlier than the first event.
   * @param callback     - Function that will be called with the Fish’s state `S`.
   *                       As long as the first event does not exist, this callback will also not be called.
   *
   * @returns              A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeOne<ESeed, S>(
    // Expression to find the Fish’s starting event, e.g. Tag('task-created').withId('my-task')
    seedEventSelector: Where<ESeed>,
    makeFish: (seedEvent: ESeed) => Fish<S, any>,
    callback: (newState: S) => void,
    stoppedByError?: (err: unknown) => void,
  ): CancelSubscription

  /* CONDITIONAL EMISSION (STATE EFFECTS) */

  /**
   * Run a `StateEffect` against currently known local state of Fish. Emit events based on it by
   * calling the `enqueue` function passed into the invocation of your effect. Every subsequent
   * invocation of `run` for the same Fish is guaranteed to see all events previously enqueued by
   * effects on that Fish already applied to the state. (Local serialisation guarantee.)
   *
   * In regards to other nodes or Fishes, there are no serialisation guarantees.
   *
   * @typeParam S              - State of the Fish, input value to the effect.
   * @typeParam EWrite         - Event type(s) the effect may emit.
   *
   * @param fish       - Complete Fish information.
   * @param effect     - Function to enqueue new events based on state.
   * @returns            A `PendingEmission` object that can be used to register callbacks with the effect’s completion.
   */
  run<S, EWrite>(fish: Fish<S, any>, fn: StateEffect<S, EWrite>): PendingEmission

  /**
   * Install a StateEffect that will be applied automatically whenever the `Fish`’s State has changed.
   * Every application will see the previous one’s resulting Events applied to the State already, if applicable;
   * but any number of intermediate States may have been skipped between two applications.
   *
   * In regards to other nodes or Fishes, there are no serialisation guarantees.
   *
   * The effect can be uninstalled by calling the returned `CancelSubscription`.
   *
   * @typeParam S              - State of the Fish, input value to the effect.
   * @typeParam EWrite         - Event type(s) the effect may emit.
   *
   * @param fish       - Complete Fish information.
   * @param effect     - Function that decides whether to enqueue new events based on the current state.
   * @param autoCancel - Condition on which the automatic effect will be cancelled -- state on which `autoCancel` returns `true`
   *                     will be the first state the effect is *not* applied to anymore. Keep in mind that not all intermediate
   *                     states will be seen by this function.
   * @returns            A `CancelSubscription` object that can be used to cancel the automatic effect.
   */
  keepRunning<S, EWrite>(
    fish: Fish<S, any>,
    fn: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription

  /* HOUSE KEEPING FUNCTIONS */

  /**
   * Dispose of this Pond, stopping all underlying async operations.
   */
  dispose(): void

  /**
   * Information about the current pond
   */
  info(): PondInfo

  /**
   * Register a callback invoked whenever the Pond’s state changes.
   * The `PondState` is a general description of activity within the Pond internals.
   */
  getPondState(callback: (newState: PondState) => void): CancelSubscription

  /**
   * Register a callback invoked whenever the node’s connectivity status changes.
   */
  getNodeConnectivity(params: GetNodeConnectivityParams): CancelSubscription

  /**
   * Wait for the node to get in sync with the swarm.
   * It is strongly recommended that any interaction with the Pond is delayed until the onSyncComplete callback has been notified.
   * To obtain progress information about the sync, the onProgress callback can be supplied.
   */
  waitForSwarmSync(params: WaitForSwarmSyncParams): void
}

type ActiveObserveAll<S> = Readonly<{
  states: Subject<S[]>
  subscription: Subscription
}>

const getOrInitialize = <T>(
  cache: Record<string, { states: Subject<T>; subscription: Subscription }>,
  key: string,
  makeT: () => Observable<T>,
) => {
  const existing = cache[key]
  if (existing !== undefined) {
    return existing
  }

  const stateSubject = new ReplaySubject<T>(1, undefined, Scheduler.queue)
  const subscription = makeT().subscribe(stateSubject)

  const a = {
    states: stateSubject,
    subscription,
  }
  cache[key] = a
  return a
}

const defaultReportFishError: FishErrorReporter = (err, fishId, detail) =>
  console.error('Error while executing', FishId.canonical(fishId), ':', err, detail)

class Pond2Impl implements Pond {
  readonly hydrateV2: <S, E>(
    subscriptionSet: Where<E>,
    initialState: S,
    onEvent: (state: S, event: E, metadata: Metadata) => S,
    fishId: FishId,
    isReset?: IsReset<E>,
    deserializeState?: (jsonState: unknown) => S,
  ) => Observable<StateWithProvenance<S>>

  readonly observeAllImpl: <ESeed, S>(
    firstEvents: Where<ESeed>,
    makeFish: (seed: ESeed) => Fish<S, any> | undefined,
    expireAfterSeed?: Milliseconds,
  ) => Observable<StartedFishMap<S>>

  activeFishes: {
    [fishId: string]: ActiveFish<any>
  } = {}

  activeObserveAll: Record<string, ActiveObserveAll<any>> = {}

  constructor(
    private readonly eventStore: EventStore,
    private readonly snapshotStore: SnapshotStore,
    private readonly pondStateTracker: PondStateTracker,
    private readonly monitoring: Monitoring,
    private readonly finalTeardown: () => void,
    private readonly opts: PondOptions,
  ) {
    this.hydrateV2 = observeMonotonic(
      this.eventStore,
      this.snapshotStore,
      SnapshotScheduler.create(10),
      typeof opts.fishErrorReporter === 'function'
        ? opts.fishErrorReporter
        : defaultReportFishError,
      this.pondStateTracker,
    )

    this.observeAllImpl = FishJar.observeAll(
      this.eventStore,
      this.snapshotStore,
      this.pondStateTracker,
    )
  }

  getPondState = (callback: (newState: PondState) => void) => {
    const sub = this.pondStateTracker.observe().subscribe(callback)
    return () => sub.unsubscribe()
  }

  getNodeConnectivity = (params: GetNodeConnectivityParams) => {
    const sub = this.eventStore
      .connectivityStatus(
        params.specialSources || [],
        this.opts.hbHistDelay || 1e12,
        this.opts.updateConnectivityEvery || Milliseconds.of(10_000),
        this.opts.currentPsnHistoryDelay || 6,
      )
      .subscribe(params.callback)

    return () => sub.unsubscribe()
  }

  waitForSwarmSync = (params: WaitForSwarmSyncParams) => {
    const splash = streamSplashState(this.eventStore, params).finally(params.onSyncComplete)

    if (params.onProgress) {
      splash.subscribe(params.onProgress)
    } else {
      splash.subscribe()
    }
  }

  info = () => {
    return {
      nodeId: this.eventStore.nodeId,
    }
  }

  dispose = () => {
    this.monitoring.dispose()

    Object.values(this.activeFishes).forEach(({ subscription }) => subscription.unsubscribe())

    Object.values(this.activeObserveAll).forEach(({ subscription }) => subscription.unsubscribe())

    this.finalTeardown()
  }

  /* POND V2 FUNCTIONS */
  private emitTagged0 = <E>(emit: ReadonlyArray<Emit<E>>): Observable<Events> => {
    const events = emit.map(({ tags, payload }) => {
      const timestamp = Timestamp.now()

      const event: UnstoredEvent = {
        tags: isTyped(tags) ? tags.rawTags : tags,
        timestamp,
        payload,
      }

      return event
    })

    return this.eventStore.persistEvents(events)
  }

  emit = <E>(tags: Tags<E>, payload: E): PendingEmission => {
    return this.emitMany({ tags, payload })
  }

  private emitMany = (...emissions: ReadonlyArray<Emit<any>>): PendingEmission => {
    // `shareReplay` so that every piece of user code calling `subscribe`
    // on the return value will actually be executed
    const o = this.emitTagged0(emissions)
      .mapTo(void 0)
      .shareReplay(1)

    // Maybe TODO: Subscribing here causes the request to be cancelled too early. What is the problem?
    // o.subscribe()

    return pendingEmission(o)
  }

  private getCachedOrInitialize = <S, E>(
    subscriptionSet: Where<E>,
    initialState: S,
    onEvent: Reduce<S, E>,
    fishId: FishId,
    isReset: IsReset<E> | undefined,
    deserializeState: ((jsonState: unknown) => S) | undefined,
  ): ActiveFish<S> => {
    const key = FishId.canonical(fishId)
    return getOrInitialize(this.activeFishes, key, () =>
      this.hydrateV2(subscriptionSet, initialState, onEvent, fishId, isReset, deserializeState),
    )
  }

  private observeTagBased0 = <S, E>(acc: Fish<S, E>): ActiveFish<S> => {
    return this.getCachedOrInitialize(
      acc.where,
      acc.initialState,
      acc.onEvent,
      acc.fishId,
      acc.isReset,
      acc.deserializeState,
    )
  }

  observe = <S, E>(
    fish: Fish<S, E>,
    callback: (newState: S) => void,
    stoppedByError?: (err: unknown) => void,
  ): CancelSubscription => {
    return omitObservable(stoppedByError, callback, this.observeTagBased0<S, E>(fish).states)
  }

  // Get a (cached) Handle to run StateEffects against. Every Effect will see the previous one applied to the State.
  private run0 = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
  ): ((effect: StateEffectInternal<S, EWrite>) => PendingEmission) => {
    const cached = this.observeTagBased0(agg)
    const handleInternal = this.getOrCreateCommandHandle0(agg, cached)

    return effect => pendingEmission(handleInternal(effect))
  }

  private v2CommandHandler = (emit: EmissionRequest<unknown>) => {
    return Observable.from(Promise.resolve(emit)).mergeMap(x => this.emitTagged0(x))
  }

  private getOrCreateCommandHandle0 = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    cached: ActiveFish<S>,
  ): ((effect: StateEffectInternal<S, EWrite>) => Observable<void>) => {
    const handler = this.v2CommandHandler

    const commandPipeline =
      cached.commandPipeline ||
      FishJar.commandPipeline<S, EmissionRequest<any>>(
        this.pondStateTracker,
        agg.fishId.entityType,
        agg.fishId.name,
        handler,
        cached.states,
        toEventPredicate(agg.where),
      )
    cached.commandPipeline = commandPipeline

    return effect => {
      const o = new Observable<void>(x =>
        commandPipeline.subject.next({
          type: 'command',
          command: effect,
          onComplete: () => {
            x.next()
            x.complete()
          },
          onError: (err: any) => x.error(err),
        }),
      )
        .shareReplay(1)
        // Subscribing on Scheduler.queue is not strictly required, but helps with dampening feedback loops
        .subscribeOn(Scheduler.queue)

      // We just subscribe to guarantee effect application;
      // user is responsible for handling errors on the returned object if desired.
      o.catch(() => Observable.empty()).subscribe()

      return o
    }
  }

  observeAll = <ESeed, S>(
    // Expression to extract the initial events, e.g. Tag<TaskCreated>
    seedEvents: Where<ESeed>,
    // Create a concrete Fish from the initial event
    makeFish: (seed: ESeed) => Fish<S, any> | undefined,
    // When to remove Fish from the "all" set.
    opts: ObserveAllOpts,
    callback: (states: S[]) => void,
  ): CancelSubscription => {
    const safeMakeFish = (seed: ESeed) => {
      try {
        return makeFish(seed)
      } catch (err) {
        // Maybe improve me at some point
        log.pond.error('Swallowed makeFish error:', err)
        return undefined
      }
    }

    const makeAgg = (): Observable<S[]> => {
      const fishStructs$ = this.observeAllImpl(
        seedEvents,
        safeMakeFish,
        typeof opts.expireAfterSeed === 'number' ? opts.expireAfterSeed : opts.expireAfterFirst,
      )

      return fishStructs$.switchMap(known => {
        const observations = known
          .toArray()
          .map(([_key, fish]) =>
            this.observeTagBased0<S, any>(fish.fish).states.map(swp => swp.state),
          )

        return observations.length === 0
          ? Observable.of([])
          : Observable.combineLatest(observations)
      })
    }

    const fishStates$ = Caching.isEnabled(opts.caching)
      ? getOrInitialize(this.activeObserveAll, opts.caching.key, makeAgg).states
      : makeAgg()

    const sub = fishStates$.subscribe(callback)
    return () => sub.unsubscribe()
  }

  observeOne = <ESeed, S>(
    // Expression to find the Fish’s starting event, e.g. Tag('task-created').withId('my-task')
    seedEvent: Where<ESeed>,
    makeFish: (seedEvent: ESeed) => Fish<S, any>,
    callback: (newState: S) => void,
    stoppedByError?: (err: unknown) => void,
  ): CancelSubscription => {
    const states = this.eventStore
      .allEvents(
        {
          psns: {},
          default: 'min',
        },
        { psns: {}, default: 'max' },
        seedEvent,
        AllEventsSortOrders.Unsorted,
      )
      .first()
      .concatMap(x => x)
      .concatMap(f => this.observeTagBased0<S, unknown>(makeFish(f.payload as ESeed)).states)

    return omitObservable(stoppedByError, callback, states)
  }

  run = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    fn: StateEffect<S, EWrite>,
  ): PendingEmission => {
    const handle = this.run0(agg)
    return handle(wrapStateFn(fn))
  }

  keepRunning = <S, EWrite>(
    fish: Fish<S, any>,
    fn: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription => {
    const effect = wrapStateFn(fn)

    // We use this state `cancelled` to stop effects "asap" when user code calls the cancellation function.
    // Otherwise it might happen that we have already queued the next effect and run longer than desired.
    let cancelled = false

    const wrappedEffect = (state: S) => (cancelled ? [] : effect(state))

    const cached = this.observeTagBased0(fish)
    const states = cached.states
    const handleInternal = this.getOrCreateCommandHandle0(fish, cached)

    const tw = autoCancel
      ? (state: S) => {
          if (cancelled) {
            return false
          } else if (autoCancel(state)) {
            cancelled = true
            return false
          }

          return true
        }
      : () => !cancelled

    states
      .observeOn(Scheduler.async)
      .map(swp => swp.state)
      .takeWhile(tw)
      // We could also just use `do` instead of `mergeMap` (using the public API),
      // for no real loss, but no gain either.
      .mergeMap(() => handleInternal(wrappedEffect))
      .subscribe()

    return () => (cancelled = true)
  }
}

/**
 * All services needed by the pond
 */
type Services = Readonly<{
  eventStore: EventStore
  snapshotStore: SnapshotStore
  commandInterface: CommandInterface

  teardown: () => void
}>

const mockSetup = (): Services => {
  const eventStore = EventStore.mock()
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return { eventStore, snapshotStore, commandInterface, teardown: noop }
}

const createServices = async (multiplexer: MultiplexedWebsocket): Promise<Services> => {
  const sourceId = await getNodeId(multiplexer)
  const eventStore = EventStore.ws(multiplexer, sourceId)
  // FIXME: V2 support for snapshots
  const snapshotStore = SnapshotStore.noop
  const commandInterface = CommandInterface.ws(multiplexer, sourceId)
  return { eventStore, snapshotStore, commandInterface, teardown: multiplexer.close }
}

const mkPond = async (connectionOpts: Partial<WsStoreConfig>, opts: PondOptions): Promise<Pond> => {
  const multiplexer = await mkMultiplexer(extendDefaultWsStoreConfig(connectionOpts))
  const services = await createServices(multiplexer)
  return pondFromServices(services, opts)
}

const mkMockPond = (opts?: PondOptions): Pond => {
  const opts1: PondOptions = opts || {}
  const services = mockSetup()
  return pondFromServices(services, opts1)
}

/** A Pond with extensions for testing. @public */
export type TestPond = Pond & {
  directlyPushEvents: (events: TestEvent[]) => void
}
const mkTestPond = (opts?: PondOptions): TestPond => {
  const opts1: PondOptions = opts || {}
  const eventStore = EventStore.test(NodeId.of('TEST'))
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return {
    ...pondFromServices({ eventStore, snapshotStore, commandInterface, teardown: noop }, opts1),
    directlyPushEvents: eventStore.directlyPushEvents,
  }
}
const pondFromServices = (services: Services, opts: PondOptions): Pond => {
  const { eventStore, snapshotStore, teardown } = services

  // FIXME: V2 has no support for the Monitoring methods
  const monitoring = Monitoring.mock()

  log.pond.debug('start pond with SourceID %s from store', eventStore.nodeId)

  const pondStateTracker = mkPondStateTracker(log.pond)
  const pond: Pond2Impl = new Pond2Impl(
    eventStore,
    snapshotStore,
    pondStateTracker,
    monitoring,
    teardown,
    opts,
  )

  return pond
}

/** Static methods for constructing Pond instances. @public */
export const Pond = {
  /** Start Pond with default parameters. @public */
  default: async (): Promise<Pond> => Pond.of({}, {}),
  /** Start Pond with custom parameters. @public */
  of: mkPond,
  /** Get a Pond instance that does nothing. @public */
  mock: mkMockPond,
  /**
   * Get a Pond instance that runs a simulated store locally
   * whose contents can be manually modified.
   * @public
   */
  test: mkTestPond,
}
