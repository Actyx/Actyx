/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  Actyx,
  ActyxEvent,
  ActyxOpts,
  AppManifest,
  CancelSubscription,
  EventFns,
  Metadata,
  Milliseconds,
  NodeId,
  NodeInfo,
  PendingEmission,
  StateWithProvenance,
  TaggedEvent,
  Tags,
  TestEvent,
  TimeInjector,
  toEventPredicate,
  Where,
} from '@actyx/sdk'
import { SnapshotStore } from '@actyx/sdk/lib/snapshotStore'
import {
  Observable,
  ReplaySubject,
  asyncScheduler,
  queueScheduler,
  Subject,
  Subscription,
  lastValueFrom,
  EMPTY,
  of,
  combineLatest,
  from,
} from '../node_modules/rxjs'
import {
  catchError,
  shareReplay,
  map,
  subscribeOn,
  finalize,
  take,
  switchMap,
  concatMap,
  observeOn,
  takeWhile,
  mergeMap,
} from '../node_modules/rxjs/operators'
import { CommandPipeline, FishJar, StartedFishMap } from './fishJar'
import log from './loggers'
import { observeMonotonic } from './monotonic'
import { SnapshotScheduler } from './monotonic/snapshotScheduler'
import { mkPondStateTracker, PondState, PondStateTracker } from './pond-state'
import { SplashState, streamSplashState, WaitForSwarmConfig } from './splashState'
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
} from './types'
import { noop } from './util'

/** Advanced configuration options for the Pond. @public */
export type PondOptions = {
  /**
   * Callback that is invoked whenever Fish execution encounters an error.
   * If none is supplied, errors will be logged to the console.
   */
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
      .pipe(
        map((x) => x.state),
        // Use async scheduler to make direct cancelation work
        subscribeOn(asyncScheduler),
      )
      .subscribe(callback, typeof stoppedByError === 'function' ? stoppedByError : noop)
    return sub.unsubscribe.bind(sub)
  } catch (err) {
    stoppedByError && stoppedByError(err)
    return noop
  }
}

const wrapStateFn = <S, EWrite>(pond: Pond, fn: StateEffect<S, EWrite>) => {
  const effect = async (state: S) => {
    const emissions: Emit<any>[] = []
    let returned = false

    const enqueueEmission: AddEmission<EWrite> = (...args) => {
      if (returned) {
        throw new Error(
          'The function you passed to run/keepRunning has already returned -- enqueuing emissions via the passed "AddEmission" function is no longer possible.',
        )
      }

      if (args.length === 1) {
        const { tags, event } = args[0]
        emissions.push({ tags: Tags<EWrite>(...tags), payload: event })
      } else {
        const [tags, payload] = args
        emissions.push({ tags, payload })
      }
    }

    await fn(state, enqueueEmission, pond)
    returned = true

    return emissions
  }

  return effect
}

/** Pending command application. @public */
export type PendingCommand = {
  // Add another callback; if emission has already completed, the callback will be executed straight-away.
  subscribe: (whenEmitted: () => void) => void
  // Convert to a Promise which resolves once emission has completed.
  toPromise: () => Promise<void>

  // TODO: This should include Metadata of emitted events and/or new state after application of new events
}

// For internal use only. TODO: Cleanup.
const pendingCmd = (o: Observable<void>): PendingCommand => ({
  subscribe: o.subscribe.bind(o),
  toPromise: () => o.toPromise(),
})

type Emit<E> = {
  tags: Tags<E>
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
  callback: (newState: unknown) => void
  specialSources?: ReadonlyArray<NodeId>
}>

/** Parameter object for the `Pond.waitForSwarmSync` call. @public */
export type WaitForSwarmSyncParams = WaitForSwarmConfig &
  Readonly<{
    onSyncComplete: () => void
    onProgress?: (newState: SplashState) => void
  }>

/**
 * Main interface for interaction with the Actyx event system.
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
   *
   * @deprecated Use `publish` instead, and always await the Promise.
   */
  emit<E>(tags: Tags<E>, event: E): PendingEmission

  /**
   * Publish any number of events.
   *
   * @param events - Events to publish. Use `Tag('foo').apply(event)` to create an array of `TaggedEvent`.
   *
   * @returns        A Promise that resolves to the emitted event’s metadata.
   */
  publish(event: TaggedEvent): Promise<Metadata>
  publish(events: ReadonlyArray<TaggedEvent>): Promise<Metadata[]>

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
   * Read the current state of a Fish.
   *
   * Caching is done based on the `fishId` inside the `fish`, i.e. if a fish with the included
   * `fishId` is already known, that other Fish’s ongoing aggregation will be used instead of
   * starting a new one.
   *
   * @param fish       - Complete Fish information.
   *
   * @returns A Promise that resolves to the Fish’s latest known state. If the Fish was stopped due to an error, the Promise will reject with that error.
   */
  currentState<S, E>(fish: Fish<S, E>): Promise<S>

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
  run<S, EWrite>(fish: Fish<S, any>, fn: StateEffect<S, EWrite>): PendingCommand

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
   * Obtain information on the Actyx node. In order to save some cycles, and because the information
   * doesn’t change all that quickly, please provide a time parameter that matches your app’s
   * freshness requirements — for human consumption a couple hundred milliseconds is good enough.
   *
   * The underlying API endpoint has been added in Actyx 2.5.0, earlier versions report dummy data.
   */
  nodeInfo(maxAgeMillis: number): Promise<NodeInfo>

  /**
   * Register a callback invoked whenever the Pond’s state changes.
   * The `PondState` is a general description of activity within the Pond internals.
   */
  getPondState(callback: (newState: PondState) => void): CancelSubscription

  /**
   * Wait for the node to get in sync with the swarm.
   * It is strongly recommended that any interaction with the Pond is delayed until the onSyncComplete callback has been notified.
   * To obtain progress information about the sync, the onProgress callback can be supplied.
   */
  waitForSwarmSync(params: WaitForSwarmSyncParams): void

  /**
   * Get an object that offers a number of functions related purely to events (no Fish).
   */
  events(): EventFns
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

  const stateSubject = new ReplaySubject<T>(1, undefined, queueScheduler)
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
    private readonly actyx: Actyx,
    private readonly snapshotStore: SnapshotStore,
    private readonly pondStateTracker: PondStateTracker,
    opts: PondOptions,
  ) {
    this.hydrateV2 = observeMonotonic(
      this.actyx,
      this.snapshotStore,
      SnapshotScheduler.create(10),
      typeof opts.fishErrorReporter === 'function'
        ? opts.fishErrorReporter
        : defaultReportFishError,
      this.pondStateTracker,
    )

    this.observeAllImpl = FishJar.observeAll(this.actyx, this.pondStateTracker)
  }

  getPondState = (callback: (newState: PondState) => void) => {
    const sub = this.pondStateTracker.observe().subscribe(callback)
    return () => sub.unsubscribe()
  }

  waitForSwarmSync = (params: WaitForSwarmSyncParams) => {
    const splash = streamSplashState(this.actyx, params).pipe(finalize(params.onSyncComplete))

    if (params.onProgress) {
      splash.subscribe(params.onProgress)
    } else {
      splash.subscribe()
    }
  }

  info = () => {
    return {
      nodeId: this.actyx.nodeId,
    }
  }

  dispose = () => {
    Object.values(this.activeFishes).forEach(({ subscription }) => subscription.unsubscribe())

    Object.values(this.activeObserveAll).forEach(({ subscription }) => subscription.unsubscribe())

    this.actyx.dispose()
  }

  /* POND V2 FUNCTIONS */
  emit = <E>(tags: Tags<E>, payload: E): PendingEmission => {
    return this.actyx.emit([tags.apply(payload)])
  }

  publish = this.actyx.publish

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

  currentState = <S, E>(fish: Fish<S, E>): Promise<S> => {
    const states = this.observeTagBased0<S, E>(fish).states
    if (states.hasError) {
      return Promise.reject(states.thrownError)
    }

    return lastValueFrom(states.pipe(take(1))).then((x) => x.state)
  }

  // Get a (cached) Handle to run StateEffects against. Every Effect will see the previous one applied to the State.
  private run0 = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
  ): ((effect: StateEffectInternal<S, EWrite>) => PendingCommand) => {
    const cached = this.observeTagBased0(agg)
    const handleInternal = this.getOrCreateCommandHandle0(agg, cached)

    return (effect) => pendingCmd(handleInternal(effect))
  }

  private v2CommandHandler = async (emit: EmissionRequest<unknown>): Promise<Metadata[]> => {
    const r = await Promise.resolve(emit)

    const e = r.flatMap((x) => x.tags.apply(x.payload))

    return this.actyx.emit(e).toPromise()
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

    return (effect) => {
      const o = new Observable<void>((x) =>
        commandPipeline.subject.next({
          type: 'command',
          command: effect,
          onComplete: () => {
            x.next()
            x.complete()
          },
          onError: (err: any) => x.error(err),
        }),
      ).pipe(
        shareReplay(1),
        // Subscribing on Scheduler.queue is not strictly required, but helps with dampening feedback loops
        subscribeOn(queueScheduler),
      )

      // We just subscribe to guarantee effect application;
      // user is responsible for handling errors on the returned object if desired.
      o.pipe(catchError(() => EMPTY)).subscribe()

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

      return fishStructs$.pipe(
        switchMap((known) => {
          const observations = known
            .toArray()
            .map(([_key, fish]) =>
              this.observeTagBased0<S, any>(fish.fish).states.pipe(map((swp) => swp.state)),
            )

          return observations.length === 0 ? of([]) : combineLatest(observations)
        }),
      )
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
    let cancelInitialSubscription: CancelSubscription | undefined = undefined

    const initial = new Promise<ActyxEvent>((resolve) => {
      cancelInitialSubscription = this.actyx.subscribe(
        {
          query: seedEvent,
        },
        resolve,
      )
    })

    const states = from(initial).pipe(
      concatMap((f) => {
        cancelInitialSubscription && cancelInitialSubscription()
        return this.observeTagBased0<S, unknown>(makeFish(f.payload as ESeed)).states
      }),
    )

    return omitObservable(stoppedByError, callback, states)
  }

  run = <S, EWrite, ReadBack = false>(
    agg: Fish<S, ReadBack extends true ? EWrite : any>,
    fn: StateEffect<S, EWrite>,
  ): PendingCommand => {
    const handle = this.run0(agg)
    return handle(wrapStateFn(this, fn))
  }

  keepRunning = <S, EWrite>(
    fish: Fish<S, any>,
    fn: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription => {
    const effect = wrapStateFn(this, fn)

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
      .pipe(
        observeOn(asyncScheduler),
        map((swp) => swp.state),
        takeWhile(tw),
        // We could also just use `do` instead of `mergeMap` (using the public API),
        // for no real loss, but no gain either.
        mergeMap(() => handleInternal(wrappedEffect)),
      )
      .subscribe()

    return () => (cancelled = true)
  }

  events = () => this.actyx

  nodeInfo(maxAgeMillis: number): Promise<NodeInfo> {
    return this.actyx.nodeInfo(maxAgeMillis)
  }
}

const mkPond = async (
  manifest: AppManifest,
  connectionOpts: ActyxOpts,
  opts: PondOptions,
): Promise<Pond> => {
  const actyx = await Actyx.of(manifest, connectionOpts)
  return pondFromServices(actyx, opts)
}

/** A Pond with extensions for testing. @public */
export type TestPond = Pond & {
  directlyPushEvents: (events: TestEvent[]) => void
}
export type TestPondOptions = PondOptions & { timeInjector?: TimeInjector }

const mkTestPond = (opts?: TestPondOptions): TestPond => {
  const opts1: TestPondOptions = opts || {}
  const actyx = {
    ...Actyx.test({ nodeId: NodeId.of('TEST'), timeInjector: opts1.timeInjector }),
    waitForSync: async () => {
      /* noop */
    },
    nodeInfo: async () =>
      new NodeInfo({ connectedNodes: 0, version: '2.0.0-test', uptime: { secs: 0, nanos: 0 } }),
  }
  return {
    ...pondFromServices(actyx, opts1),
    directlyPushEvents: actyx.directlyPushEvents,
  }
}
const pondFromServices = (actyx: Actyx, opts: PondOptions): Pond => {
  log.pond.debug('start pond with SourceID %s from store', actyx.nodeId)

  const pondStateTracker = mkPondStateTracker(log.pond)
  const pond: Pond2Impl = new Pond2Impl(actyx, actyx.snapshotStore, pondStateTracker, opts)

  return pond
}

/** Static methods for constructing Pond instances. @public */
export const Pond = {
  /** Start Pond with default parameters. @public */
  default: async (manifest: AppManifest): Promise<Pond> => Pond.of(manifest, {}, {}),
  /** Start Pond with custom parameters. @public */
  of: mkPond,
  /**
   * Get a Pond instance that runs a simulated store locally
   * whose contents can be manually modified.
   * @public
   */
  test: mkTestPond,
}
