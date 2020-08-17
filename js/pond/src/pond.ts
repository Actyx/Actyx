/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable, Scheduler } from 'rxjs'
import { CommandInterface } from './commandInterface'
import { EventStore, WsStoreConfig } from './eventstore'
import { MultiplexedWebsocket } from './eventstore/multiplexedWebsocket'
import { TestEventStore } from './eventstore/testEventStore'
import { ConnectivityStatus, Events, UnstoredEvents } from './eventstore/types'
import { extendDefaultWsStoreConfig, mkMultiplexer } from './eventstore/utils'
import { getSourceId } from './eventstore/websocketEventStore'
import { CommandPipeline, FishJar } from './fishJar'
import log from './loggers'
import { mkPondStateTracker, PondState, PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { Config as WaitForSwarmConfig, SplashState } from './splashState'
import { Monitoring } from './store/monitoring'
import { SubscriptionSet, subscriptionsToEventPredicate } from './subscription'
import { Tags, toSubscriptionSet } from './tagging'
import {
  CancelSubscription,
  Fish,
  FishId,
  FishName,
  IsReset,
  Metadata,
  Milliseconds,
  PendingEmission,
  Reduce,
  Semantics,
  Source,
  SourceId,
  StateEffect,
  StateWithProvenance,
  Timestamp,
} from './types'

const isTyped = (e: ReadonlyArray<string> | Tags<unknown>): e is Tags<unknown> => {
  return !Array.isArray(e)
}

export type PondOptions = {
  hbHistDelay?: number
  currentPsnHistoryDelay?: number
  updateConnectivityEvery?: Milliseconds

  stateEffectDebounce?: number
}

export type PondInfo = {
  sourceId: SourceId
}

export const makeEventChunk = <E>(source: Source, events: ReadonlyArray<E>): UnstoredEvents => {
  const timestamp = Timestamp.now()
  const { semantics, name } = source
  return events.map(payload => ({
    semantics,
    name,
    tags: [],
    timestamp,
    payload,
  }))
}

const omitObservable = <S>(
  callback: (newState: S) => void,
  states: Observable<StateWithProvenance<S>>,
): CancelSubscription => {
  const sub = states.map(x => x.state).subscribe(callback)
  return sub.unsubscribe.bind(sub)
}

const wrapStateFn = <S, EWrite>(fn: StateEffect<S, EWrite>) => {
  const effect = async (state: S) => {
    const emissions: Emit<any>[] = []
    await fn(state, (tags, payload) => emissions.push({ tags, payload }))
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
  readonly states: Observable<StateWithProvenance<S>>
  commandPipeline?: CommandPipeline<S, EmissionRequest<any>>
}

export type GetNodeConnectivityParams = Readonly<{
  callback: (newState: ConnectivityStatus) => void
  specialSources?: ReadonlyArray<SourceId>
}>

export type WaitForSwarmSyncParams = Readonly<{
  onSyncComplete: () => void
  onProgress?: (newState: SplashState) => void
  config?: WaitForSwarmConfig
}>

export type Pond = {
  /* EMISSION */

  /**
   * Emit a single event directly.
   *
   * @typeParam E    Type of the event payload. If your tags are statically declared,
   *                 their type will be checked against the payload’s type.
   *
   * @param tags     Tags to attach to the event. E.g. `Tags('myTag', 'myOtherTag')`
   * @param event    The event itself.
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
   * @param fish         Complete Fish information.
   * @param callback     Function that will be called whenever a new state becomes available.
   * @returns            A function that can be called in order to cancel the aggregation.
   */
  observe<S, E>(fish: Fish<S, E>, callback: (newState: S) => void): CancelSubscription

  /* CONDITIONAL EMISSION (STATE EFFECTS) */

  /**
   * Run a `StateEffect` against currently known local state of Fish. Emit events based on it by
   * calling the `enqueue` function passed into the invocation of your effect. Every subsequent
   * invocation of `run` for the same Fish is guaranteed to see all events previously enqueued by
   * effects on that Fish already applied to the state. (Local serialisation guarantee.)
   *
   * In regards to other nodes or Fishes, there are no serialisation guarantees.
   *
   * @typeParam S                State of the Fish, input value to the effect.
   * @typeParam EWrite           Event type(s) the effect may emit.
   *
   * @param fish         Complete Fish information.
   * @param effect       Function to enqueue new events based on state.
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
   * @typeParam S                State of the Fish, input value to the effect.
   * @typeParam EWrite           Event type(s) the effect may emit.
   *
   * @param fish         Complete Fish information.
   * @param effect       Function that decides whether to enqueue new events based on the current state.
   * @param autoCancel   Condition on which the automatic effect will be cancelled -- state on which `autoCancel` returns `true`
   *                     will be the first state the effect is *not* applied to anymore. Keep in mind that not all intermediate
   *                     states will be seen by this function.
   * @returns            A `CancelSubscription` object that can be used to cancel the automatic effect.
   */
  keepRunning<S, EWrite>(
    fish: Fish<S, any>,
    fn: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ): CancelSubscription

  /*
   * HOUSE KEEPING FUNCTIONS
   */

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

export class Pond2Impl implements Pond {
  readonly hydrateV2: <S, E>(
    subscriptionSet: SubscriptionSet,
    initialState: S,
    onEvent: (state: S, event: E, metadata: Metadata) => S,
    fishId: FishId,
    isReset?: IsReset<E>,
    deserializeState?: (jsonState: unknown) => S,
  ) => Observable<StateWithProvenance<S>>

  activeFishes: {
    [fishId: string]: ActiveFish<any>
  } = {}

  constructor(
    private readonly eventStore: EventStore,
    private readonly snapshotStore: SnapshotStore,
    private readonly pondStateTracker: PondStateTracker,
    private readonly monitoring: Monitoring,
    private readonly opts: PondOptions,
  ) {
    this.hydrateV2 = FishJar.hydrateV2(this.eventStore, this.snapshotStore, this.pondStateTracker)
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
    const splash = SplashState.of(this.eventStore, params.config || {}).finally(
      params.onSyncComplete,
    )

    if (params.onProgress) {
      splash.subscribe(params.onProgress)
    } else {
      splash.subscribe()
    }
  }

  info = () => {
    return {
      sourceId: this.eventStore.sourceId,
    }
  }

  dispose = () => {
    this.monitoring.dispose()
    // TODO: Implement cleanup of active fishs
  }

  /* POND V2 FUNCTIONS */
  private emitTagged0 = <E>(emit: ReadonlyArray<Emit<E>>): Observable<Events> => {
    const events = emit.map(({ tags, payload }) => {
      const timestamp = Timestamp.now()

      const event = {
        semantics: Semantics.none,
        name: FishName.none,
        tags: isTyped(tags) ? tags.toWireFormat().tags : tags,
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

    // `o` is already (probably) hot, but we subscribe just in case.
    o.subscribe()

    return pendingEmission(o)
  }

  private getCachedOrInitialize = <S, E>(
    subscriptionSet: SubscriptionSet,
    initialState: S,
    onEvent: Reduce<S, E>,
    fishId: FishId,
    isReset: IsReset<E> | undefined,
    deserializeState: ((jsonState: unknown) => S) | undefined,
  ): ActiveFish<S> => {
    const key = FishId.canonical(fishId)
    const existing = this.activeFishes[key]
    if (existing !== undefined) {
      return {
        ...existing,
        states: existing.states.observeOn(Scheduler.queue),
      }
    }

    const stateSubject = this.hydrateV2(
      subscriptionSet,
      initialState,
      onEvent,
      fishId,
      isReset,
      deserializeState,
    ).shareReplay(1)

    const a = {
      states: stateSubject,
    }
    this.activeFishes[key] = a
    return a
  }

  private observeTagBased0 = <S, E>(acc: Fish<S, E>): ActiveFish<S> => {
    return this.getCachedOrInitialize(
      toSubscriptionSet(acc.where),
      acc.initialState,
      acc.onEvent,
      acc.fishId,
      acc.isReset,
      acc.deserializeState,
    )
  }

  observe = <S, E>(fish: Fish<S, E>, callback: (newState: S) => void): CancelSubscription => {
    return omitObservable(callback, this.observeTagBased0<S, E>(fish).states)
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

    const subscriptionSet = toSubscriptionSet(agg.where)

    const commandPipeline =
      cached.commandPipeline ||
      FishJar.commandPipeline<S, EmissionRequest<any>>(
        this.pondStateTracker,
        this.eventStore.sourceId,
        agg.fishId.entityType,
        agg.fishId.name,
        handler,
        cached.states,
        subscriptionsToEventPredicate(subscriptionSet),
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
      .map(swp => swp.state)
      .takeWhile(tw)
      .debounceTime(0)
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
}>

const mockSetup = (): Services => {
  const eventStore = EventStore.mock()
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return { eventStore, snapshotStore, commandInterface }
}

const createServices = async (multiplexer: MultiplexedWebsocket): Promise<Services> => {
  const sourceId = await getSourceId(multiplexer)
  const eventStore = EventStore.ws(multiplexer, sourceId)
  const snapshotStore = SnapshotStore.ws(multiplexer)
  const commandInterface = CommandInterface.ws(multiplexer, sourceId)
  return { eventStore, snapshotStore, commandInterface }
}

const mkPond = async (connectionOpts: Partial<WsStoreConfig>, opts: PondOptions): Promise<Pond> => {
  const multiplexer = mkMultiplexer(extendDefaultWsStoreConfig(connectionOpts))
  const services = await createServices(multiplexer || mkMultiplexer())
  return pondFromServices(services, opts)
}

const mkMockPond = async (opts?: PondOptions): Promise<Pond> => {
  const opts1: PondOptions = opts || {}
  const services = mockSetup()
  return pondFromServices(services, opts1)
}

export type TestPond2 = Pond & {
  directlyPushEvents: (events: Events) => void
  eventStore: TestEventStore
}
const mkTestPond = async (opts?: PondOptions): Promise<TestPond2> => {
  const opts1: PondOptions = opts || {}
  const eventStore = EventStore.test(SourceId.of('TEST'))
  const snapshotStore = SnapshotStore.inMem()
  const commandInterface = CommandInterface.mock()
  return {
    ...pondFromServices({ eventStore, snapshotStore, commandInterface }, opts1),
    directlyPushEvents: eventStore.directlyPushEvents,
    eventStore,
  }
}
const pondFromServices = (services: Services, opts: PondOptions): Pond => {
  const { eventStore, snapshotStore, commandInterface } = services

  const monitoring = Monitoring.of(commandInterface, 10000)

  log.pond.debug('start pond with SourceID %s from store', eventStore.sourceId)

  const pondStateTracker = mkPondStateTracker(log.pond)
  const pond: Pond2Impl = new Pond2Impl(
    eventStore,
    snapshotStore,
    pondStateTracker,
    monitoring,
    opts,
  )

  return pond
}

export const Pond = {
  default: async (): Promise<Pond> => Pond.of({}, {}),
  of: mkPond,
  mock: mkMockPond,
  test: mkTestPond,
}
