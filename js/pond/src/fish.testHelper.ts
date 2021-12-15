/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import {
  Actyx,
  EventKey,
  Lamport,
  LocalSnapshot,
  NodeId,
  Offset,
  OffsetMap,
  Tag,
  TestEvent,
  Timestamp,
  Where,
} from '@actyx/sdk'
import { SnapshotStore } from '@actyx/sdk/lib/snapshotStore'
import { last } from 'ramda'
import { Observable, from, lastValueFrom, of, asyncScheduler } from '../node_modules/rxjs'
import {
  concatWith,
  concatMap,
  concatMapTo,
  map,
  shareReplay,
  observeOn,
  first,
  debounceTime,
  take,
} from '../node_modules/rxjs/operators'
import { Fish, FishId } from '.'
import { observeMonotonic } from './monotonic'
import { minSnapshotAge, SnapshotScheduler } from './monotonic/snapshotScheduler'
import { FishErrorContext, FishErrorReporter, FishName, Semantics, SnapshotFormat } from './types'

export type NumberFishEvent = number | 'padding'
export type NumberFishState = number[]

export type RawEvent = Readonly<{
  payload: NumberFishEvent
  timestamp: number
  source: string
  tags?: string[]
}>

export type LastPublished = Readonly<{
  timestamp: number
  psn: number
  sequence: number
}>

export const testFishSemantics = Semantics.of('test-semantics')
export const testFishName = FishName.of('test-fishname')

export const testFishId = FishId.of(testFishSemantics, testFishName, 1)

export type EventFactory = {
  mkEvent: (raw: RawEvent) => TestEvent[]
  mkEvents: (raw: RawEvent[]) => TestEvent[]
}

export const eventFactory = () => {
  const lastPublishedForSources: Record<string, LastPublished> = {}

  const mkEvent: (raw: RawEvent) => TestEvent = (raw) => {
    const lastPublished = lastPublishedForSources[raw.source]
    // Choosing random starting psns for sources should never change test outcomes.
    const offset = lastPublished ? lastPublished.psn : Math.round(Math.random() * 1000)

    if (lastPublished && raw.timestamp < lastPublished.timestamp) {
      throw new Error('A single source will never timetravel, please review your test scenario.')
    }

    const fullEvent = {
      timestamp: Timestamp.of(raw.timestamp),
      stream: NodeId.of(raw.source),
      lamport: Lamport.of(raw.timestamp),
      offset: Offset.of(offset),
      payload: raw.payload,
      semantics: testFishSemantics,
      name: testFishName,
      tags: (raw.tags || []).concat(['default']),
    }

    lastPublishedForSources[raw.source] = {
      timestamp: raw.timestamp,
      psn: offset + 1,
      sequence: offset + 1,
    }

    return fullEvent
  }

  const mkEvents: (raw: RawEvent[]) => TestEvent[] = (raw) => raw.map(mkEvent)

  return {
    mkEvent,
    mkEvents,
  }
}

export const mkNumberFish = (
  semanticSnapshot?: (ev: NumberFishEvent) => boolean,
  where: Where<NumberFishEvent> = Tag('default'),
): Fish<NumberFishState, NumberFishEvent> => ({
  where,
  initialState: [],
  fishId: testFishId,
  onEvent: (state, payload) => {
    if (payload === 'padding') {
      return state
    }

    // The fish should enforce consistency between the predicate and actual fish behaviour.
    // This is what it means for something to be semantic snapshot. If the fish does not return to some initial state
    // for the event that is matched by the semanticSnapshot predicate, the behaviour of the FES is not guaranteed to be correct.

    // However, for test purposes (unlike ipfsStore.replay.spec), we have created an evil fish, that does not enforce consistency.
    state.push(payload)
    return state
  },
  isReset: semanticSnapshot,
})

export type SnapshotData = Readonly<{
  semantics: Semantics
  name: FishName
  key: EventKey
  offsets: OffsetMap
  horizon: EventKey | undefined
  cycle: number
  version: number
  tag: string
  blob: string
}>

export const mkSnapshot = (
  state: NumberFishState,
  time: number,
  horizon?: number,
  offsetMap?: Record<string, Offset>,
  version?: number,
): SnapshotData => {
  const offsets = offsetMap ? offsetMap : { 'some-source': Offset.of(1) }

  return {
    semantics: testFishSemantics,
    name: testFishName,
    key: { offset: 1, stream: 'some-source', lamport: time } as EventKey,
    offsets,
    horizon: horizon
      ? ({ offset: 0, stream: 'some-source', lamport: horizon } as EventKey)
      : undefined,
    cycle: 1,
    version: version || 1,
    tag: 'year',
    blob: JSON.stringify(state),
  }
}

export const snapshotTestSetup = async <S>(
  fish: Fish<S, NumberFishEvent>,
  storedEvents?: ReadonlyArray<TestEvent>,
  storedSnapshots?: ReadonlyArray<SnapshotData>,
) => {
  const sourceId = NodeId.of('LOCAL-test-source')
  const actyx = Actyx.test({ nodeId: sourceId })
  if (storedEvents) actyx.directlyPushEvents(storedEvents)

  let lastErr: FishErrorContext | null = null
  const testReportFishError: FishErrorReporter = (_err, _fishId, detail) => {
    lastErr = detail
  }
  const latestErr = () => lastErr

  const snapshotStore = SnapshotStore.noop // actyx.snapshotStore
  await lastValueFrom(
    from(storedSnapshots || []).pipe(
      concatMap((snap) => {
        return snapshotStore.storeSnapshot(
          snap.semantics,
          snap.name,
          snap.key,
          snap.offsets,
          snap.horizon,
          snap.cycle,
          snap.version,
          snap.tag,
          snap.blob,
        )
      }),
      concatMapTo([]),
      concatWith(of(undefined)),
    ),
  )

  const hydrate = observeMonotonic(
    actyx,
    snapshotStore,
    SnapshotScheduler.create(10),
    testReportFishError,
  )

  const observe = hydrate(
    fish.where,
    fish.initialState,
    fish.onEvent,
    fish.fishId,
    fish.isReset,
    fish.deserializeState,
  ).pipe(
    map((x) => x.state),
    shareReplay(1),
  )

  const pubEvents = actyx.directlyPushEvents

  const applyAndGetState = async (events: ReadonlyArray<TestEvent>) => {
    // adding events may or may not emit a new state, depending on whether the events
    // were relevant (might be before semantic snapshot or duplicates)
    const pubProm = lastValueFrom(observe.pipe(observeOn(asyncScheduler), debounceTime(0), first()))
    pubEvents(events)
    return pubProm
  }

  const latestSnap = async () =>
    snapshotStore.retrieveSnapshot('test-semantics', 'test-fishname', 1).then((x) => {
      const c = x as LocalSnapshot<string> | undefined
      return c ? { ...c, state: JSON.parse(c.state) } : undefined
    })

  const wakeup = () => lastValueFrom(observe.pipe(take(1)))

  return {
    latestSnap,
    latestErr,
    snapshotStore,
    applyAndGetState,
    observe,
    pubEvents,
    wakeup,
  }
}

export const semanticSnap: (ev: NumberFishEvent) => boolean = (payload) => payload === -1

export const localSnap = (version: number): SnapshotFormat<NumberFishState, NumberFishState> =>
  SnapshotFormat.identity(version)

export type FishTestFn = (fish: Fish<NumberFishState, NumberFishEvent>) => Promise<void>

export type TestFish = [string, Fish<NumberFishState, NumberFishEvent>]

export const forFishes =
  (...fishesToTest: TestFish[]) =>
  (what: string, fishTest: FishTestFn) => {
    for (const testFish of fishesToTest) {
      it('fish ' + testFish[0] + ' ' + what, async () => fishTest(testFish[1]))
    }
  }

type MkNumberEvent = {
  val: number
  source: string
  tAdd: (t: Timestamp) => Timestamp
}

type MkPadding = {
  numEvents: number
  source: string
  tAdd: (t: Timestamp) => Timestamp
}

export type MkEvent = MkNumberEvent | MkPadding

// eslint-disable-next-line no-prototype-builtins
const isPadding = (e: MkEvent): e is MkPadding => e.hasOwnProperty('numEvents')

const incrementBy = (delta: number) => (t: Timestamp) => Timestamp.of(t + delta)

export type SnapshotStride = 'large' | 'medium' | 'small'

const strideToExponent = (stride: SnapshotStride) => {
  switch (stride) {
    case 'large':
      return 14
    case 'medium':
      return 12
    case 'small':
      return 10
  }
}

export const emitter = (source: string) => {
  const r = (val: number) => ({
    val,
    source,
    tAdd: incrementBy(100),
  })

  r.triggerLocalSnapshot = (stride: SnapshotStride = 'small') => ({
    numEvents: Math.pow(2, strideToExponent(stride)),
    source,
    tAdd: incrementBy(10_000),
  })

  r.ageSnapshotsOverMinAge = () => ({
    numEvents: 1,
    source,
    // Events need to have faded a certain time into the past to be eligible for snapshotting.
    tAdd: incrementBy(minSnapshotAge + 100),
  })

  return r
}

export type Timeline = {
  all: TestEvent[]
  of: (...sources: string[]) => TestEvent[]
}

export const mkTimeline = (...events: MkEvent[]): Timeline => {
  const { mkEvent } = eventFactory()

  let t = Timestamp.of(100)
  const timeline: TestEvent[] = []

  for (const e of events) {
    t = e.tAdd(t)

    if (isPadding(e)) {
      for (let i = 0; i < e.numEvents; i++) {
        timeline.push(
          mkEvent({
            payload: 'padding',
            timestamp: t,
            source: e.source,
          }),
        )

        t = e.tAdd(t)
      }
    } else {
      timeline.push(
        mkEvent({
          payload: e.val,
          timestamp: t,
          source: e.source,
        }),
      )
    }
  }

  return {
    all: timeline,
    of: (...sources: string[]) => timeline.filter((ev) => sources.includes(ev.stream)),
  }
}

export const offsets = (...eventsBySource: TestEvent[][]) => {
  return eventsBySource.reduce((acc, events) => {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const lastEvent = last(events)!
    return {
      ...acc,
      [lastEvent.stream]: lastEvent.offset,
    }
  }, {})
}
