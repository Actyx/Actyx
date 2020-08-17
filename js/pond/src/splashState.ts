/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as immutable from 'immutable'
import { Observable, Subject } from 'rxjs'
import { EventStore, OffsetMap } from './eventstore'
import { NodeInfoEntry, SwarmInfo, SwarmSummary } from './store/swarmState'
import { takeWhileInclusive } from './util'

export type FullWaitForSwarmConfig = Readonly<{
  /**
   * Splash screen enabled
   */
  enabled: boolean
  /**
   * Delay until we consider that we got the swarm state
   */
  waitForSwarmMs: number
  /**
   * Minimum number of sources until we consider that we got the swarm state
   */
  minSources: number
  /**
   * Delay that we allow to sync with the swarm state (leave undefined to wait indefinitely)
   */
  waitForSyncMs?: number
  /**
   * True if we allow the user to skip the splash screen
   */
  allowSkip: boolean
}>

const defaults: FullWaitForSwarmConfig = {
  enabled: true,
  waitForSwarmMs: 10000,
  minSources: 0,
  allowSkip: true,
}

export type WaitForSwarmConfig = Partial<FullWaitForSwarmConfig>

export const WaitForSwarmConfig = {
  defaults,
}

export type Progress = Readonly<{ min: number; current: number; max: number }>

export type SyncProgress = Readonly<{
  sources: Progress
  events: Progress
}>

export const getSyncProgress = (current: SwarmInfo, reference: SwarmInfo): SyncProgress => {
  const r = {
    sources: {
      // nunber of relevant sources the pond had in the beginning
      min: 0,
      // number of current sources that are also in the reference swarm info
      current: 0,
      // total number of sources in the swarm in the reference swarm info
      max: 0,
    },
    events: {
      // nunber of relevant events we had in the beginning
      min: 0,
      // number of own events for the sources in the reference swarm info
      current: 0,
      // total number of events in the swarm in the reference swarm info
      max: 0,
    },
  }
  reference.nodes.forEach((ref, source) => {
    if (ref.swarm !== undefined) {
      // source exists in the swarm
      r.sources.max++
      r.events.max += ref.swarm
      const pond = current.nodes.get(source)
      if (pond !== undefined && pond.own !== undefined) {
        r.sources.current++
        r.events.current += Math.min(ref.swarm, pond.own)
      }
      if (ref.own !== undefined) {
        r.sources.min += 1
        r.events.min += Math.min(ref.swarm, ref.own)
      }
    }
  }, {})

  return r
}

const synced = (state: SplashState): boolean => {
  if (state.mode === 'discovery') {
    // we don't have the reference state yet, so we don't even know what to sync to
    return false
  }
  // wait until we got all sources (should we return false if sources === 0)
  const result =
    state.progress.sources.current === state.progress.sources.max &&
    state.progress.events.current === state.progress.events.max

  return result
}

export const getSplashStateImpl = (
  config: WaitForSwarmConfig,
  swarmInfo: Observable<SwarmInfo>,
): Observable<SplashState> => {
  const { waitForSwarmMs, waitForSyncMs, minSources, allowSkip, enabled } = {
    ...defaults,
    ...config,
  }
  if (!enabled) {
    return Observable.empty()
  }
  const userSkip = new Subject<void>()
  // emits when either the user skips or the fixed splash time has elapsed
  const skip = userSkip.merge(
    waitForSyncMs !== undefined
      ? Observable.timer(waitForSwarmMs + waitForSyncMs)
      : Observable.never(),
  )
  const initial: SplashState = {
    mode: 'discovery',
    skip: allowSkip ? () => userSkip.next(undefined) : undefined,
    current: SwarmSummary.empty,
  }
  return Observable.defer(() => {
    const t0 = Date.now()
    // true once we consider that we have enough info about the swarm, according to config
    const startSync = (current: SwarmSummary) =>
      Date.now() - t0 > waitForSwarmMs && current.sources.swarm >= minSources
    const scanner = (agg: SplashState, current: SwarmSummary): SplashState =>
      agg.mode === 'discovery'
        ? startSync(current)
          ? {
              ...agg,
              mode: 'sync',
              reference: current, // this is now our reference
              progress: getSyncProgress(current.info, current.info),
              current,
            }
          : { ...agg, current }
        : {
            ...agg,
            current,
            progress: getSyncProgress(current.info, agg.reference.info),
          }
    return swarmInfo
      .map(SwarmSummary.fromSwarmInfo)
      .startWith(SwarmSummary.empty)
      .scan<SwarmSummary, SplashState>(scanner, initial)
      .pipe(takeWhileInclusive(x => !synced(x)))
      .takeUntil(skip)
  })
}

/**
 * Discovering swarm state.
 * @public
 */
export type SplashStateDiscovery = Readonly<{
  mode: 'discovery'
  current: SwarmSummary
  skip?: () => void
}>

/**
 * Synchronizing up to the discovered swarm state.
 * @public
 */
export type SplashStateSync = Readonly<{
  mode: 'sync'
  reference: SwarmSummary
  progress: SyncProgress
  current: SwarmSummary
  skip?: () => void
}>

/**
 * Current state of swarm synchronization procedure.
 * @public
 */
export type SplashState = SplashStateDiscovery | SplashStateSync

const toSwarmInfo = ([seen, own]: [OffsetMap, OffsetMap]): SwarmInfo => {
  const allSources = [...Object.keys(seen), ...Object.keys(own)]
  const records: {
    [source: string]: NodeInfoEntry
  } = allSources.reduce(
    (acc, key) => ({
      ...acc,
      [key]: {
        own: own[key],
        swarm: seen[key],
      },
    }),
    {},
  )

  return {
    nodes: immutable.Map(records),
  }
}

export const streamSplashState = (
  eventStore: EventStore,
  config: WaitForSwarmConfig,
): Observable<SplashState> => {
  const waitForSwarmMs = config.waitForSwarmMs || defaults.waitForSwarmMs

  const highestSeenRoots$ = Observable.interval(500)
    .concatMapTo(eventStore.highestSeen())
    .takeUntil(Observable.timer(waitForSwarmMs))
    .map(x => x.psns)

  /**
   * Start with one call to present, then guarantee that at least one additional present
   * value will come in as soon as the `discovery` phase finishes so that it could transition
   * into a `sync` one.
   */
  const present$ = Observable.merge(
    eventStore.present().take(1),
    Observable.timer(waitForSwarmMs).switchMapTo(Observable.defer(eventStore.present)),
  ).map(x => x.psns)

  const swarmInfo$ = Observable.combineLatest(highestSeenRoots$, present$).map(toSwarmInfo)

  return getSplashStateImpl(config, swarmInfo$)
}
