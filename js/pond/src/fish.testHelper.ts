/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { last } from 'ramda'
import { Observable, Scheduler } from 'rxjs'
import { Fish, FishId, TestEvent } from '.'
import { EventStore } from './eventstore'
import { Event, Events, OffsetMap } from './eventstore/types'
import { observeMonotonic } from './monotonic'
import { SnapshotStore } from './snapshotStore'
import { minSnapshotAge, SnapshotScheduler } from './store/snapshotScheduler'
import { Tag, toSubscriptionSet, Where } from './tagging'
import {
  EventKey,
  FishName,
  Lamport,
  Psn,
  Semantics,
  SnapshotFormat,
  SourceId,
  Timestamp,
} from './types'

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
  mkEvent: (raw: RawEvent) => Event
  mkEvents: (raw: RawEvent[]) => Events
}

export const eventFactory = () => {
  const lastPublishedForSources: Record<string, LastPublished> = {}

  const mkEvent: (raw: RawEvent) => Event = raw => {
    const lastPublished = lastPublishedForSources[raw.source]
    // Choosing random starting psns for sources should never change test outcomes.
    const offset = lastPublished ? lastPublished.psn : Math.round(Math.random() * 1000)

    if (lastPublished && raw.timestamp < lastPublished.timestamp) {
      throw new Error('A single source will never timetravel, please review your test scenario.')
    }

    const fullEvent = {
      timestamp: Timestamp.of(raw.timestamp),
      sourceId: SourceId.of(raw.source),
      lamport: Lamport.of(raw.timestamp),
      psn: Psn.of(offset),
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

  const mkEvents: (raw: RawEvent[]) => Events = raw => raw.map(mkEvent)

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
  psnMap: OffsetMap
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
  psnMap?: Record<string, Psn>,
  version?: number,
): SnapshotData => {
  const psns = psnMap ? psnMap : { 'some-source': Psn.of(1) }

  return {
    semantics: testFishSemantics,
    name: testFishName,
    key: { psn: 1, sourceId: 'some-source', lamport: time } as EventKey,
    psnMap: psns,
    horizon: horizon
      ? ({ psn: 0, sourceId: 'some-source', lamport: horizon } as EventKey)
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
  const sourceId = SourceId.of('LOCAL-test-source')
  const eventStore = EventStore.test(sourceId)
  if (storedEvents) eventStore.directlyPushEvents(storedEvents)

  const snapshotStore = SnapshotStore.inMem()
  await Observable.from(storedSnapshots || [])
    .concatMap(snap => {
      return snapshotStore.storeSnapshot(
        snap.semantics,
        snap.name,
        snap.key,
        snap.psnMap,
        snap.horizon,
        snap.cycle,
        snap.version,
        snap.tag,
        snap.blob,
      )
    })
    .concatMapTo([])
    .concat(Observable.of(undefined))
    .toPromise()

  const hydrate = observeMonotonic(eventStore, snapshotStore, SnapshotScheduler.create(10))

  const observe = hydrate(
    toSubscriptionSet(fish.where),
    fish.initialState,
    fish.onEvent,
    fish.fishId,
    fish.isReset,
    fish.deserializeState,
  )
    .map(x => x.state)
    .shareReplay(1)

  const pubEvents = eventStore.directlyPushEvents

  const applyAndGetState = async (events: ReadonlyArray<TestEvent>) => {
    // adding events may or may not emit a new state, depending on whether the events
    // were relevant (might be before semantic snapshot or duplicates)
    const pubProm = observe
      .observeOn(Scheduler.async)
      .debounceTime(0)
      .first()
      .toPromise()
    pubEvents(events)
    return pubProm
  }

  const latestSnap = async () =>
    snapshotStore
      .retrieveSnapshot('test-semantics', 'test-fishname', 1)
      .then(x => (x ? { ...x, state: JSON.parse(x.state) } : undefined))

  // Await full hydration before tests run
  await observe.take(1).toPromise()

  return {
    latestSnap,
    snapshotStore,
    applyAndGetState,
    observe,
    pubEvents,
  }
}

export const semanticSnap: ((ev: NumberFishEvent) => boolean) = payload => payload === -1

export const localSnap = (version: number): SnapshotFormat<NumberFishState, NumberFishState> =>
  SnapshotFormat.identity(version)

export type FishTestFn = (fish: Fish<NumberFishState, NumberFishEvent>) => Promise<void>

export type TestFish = [string, Fish<NumberFishState, NumberFishEvent>]

export const forFishes = (...fishesToTest: TestFish[]) => (what: string, fishTest: FishTestFn) => {
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
  all: Events
  of: (...sources: string[]) => Events
}

export const mkTimeline = (...events: MkEvent[]): Timeline => {
  const { mkEvent } = eventFactory()

  let t = Timestamp.of(100)
  const timeline: Event[] = []

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
    of: (...sources: string[]) => timeline.filter(ev => sources.includes(ev.sourceId)),
  }
}

export const offsets = (...eventsBySource: Events[]) => {
  return eventsBySource.reduce((acc, events) => {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const lastEvent = last(events)!
    return {
      ...acc,
      [lastEvent.sourceId]: lastEvent.psn,
    }
  }, {})
}
