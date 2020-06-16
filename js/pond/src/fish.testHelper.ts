/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { last } from 'ramda'
import { Observable } from 'rxjs'
import { EventStore } from './eventstore'
import { Event, Events, OffsetMap } from './eventstore/types'
import { CommandExecutor } from './executors/commandExecutor'
import { hydrate } from './fishJar'
import { defaultTimeInjector, makeEventChunk, SendToStore } from './pond'
import { mkNoopPondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { minSnapshotAge } from './store/snapshotScheduler'
import { Subscription } from './subscription'
import {
  EventKey,
  FishName,
  FishType,
  FishTypeImpl,
  Lamport,
  OnStateChange,
  Psn,
  Semantics,
  SemanticSnapshot,
  SnapshotFormat,
  SourceId,
  Timestamp,
} from './types'

type NumberFishEvent = number | 'padding'

export type RawEvent = Readonly<{
  payload: NumberFishEvent
  timestamp: number
  source: string
  fishName?: string
}>

export type LastPublished = Readonly<{
  timestamp: number
  psn: number
  sequence: number
}>

export const testFishSemantics = Semantics.of('test-semantics')
export const testFishName = FishName.of('test-fishname')

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
      name: raw.fishName ? FishName.of(raw.fishName) : testFishName,
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
  subscriptions: (semantics: Semantics, name: FishName) => ReadonlyArray<Subscription>,
  semanticSnapshot?: SemanticSnapshot<NumberFishEvent> | undefined,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  localSnapshot?: SnapshotFormat<ReadonlyArray<number>, any> | undefined,
) =>
  FishType.of<ReadonlyArray<number>, NumberFishEvent, NumberFishEvent, ReadonlyArray<number>>({
    semantics: testFishSemantics,
    initialState: () => ({
      state: [],
      subscriptions: subscriptions(testFishSemantics, testFishName),
    }),
    onEvent: (state, ev) => {
      if (ev.payload === 'padding') {
        return state
      }

      // The fish should enforce consistency between the predicate and actual fish behaviour.
      // This is what it means for something to be semantic snapshot. If the fish does not return to some initial state
      // for the event that is matched by the semanticSnapshot predicate, the behaviour of the FES is not guaranteed to be correct.

      // However, for test purposes (unlike ipfsStore.replay.spec), we have created an evil fish, that does not enforce consistency.
      return [...state, ev.payload]
    },
    onStateChange: OnStateChange.publishPrivateState(),
    semanticSnapshot,
    localSnapshot,
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
  state: number[],
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

export const snapshotTestSetup = async (
  fish: FishTypeImpl<
    ReadonlyArray<number>,
    NumberFishEvent,
    NumberFishEvent,
    ReadonlyArray<number>
  >,
  storedEvents?: Events,
  storedSnapshots?: ReadonlyArray<SnapshotData>,
) => {
  const eventStore = EventStore.test(SourceId.of('LOCAL-test-source'))
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

  const sendToStore: SendToStore = (src, events) => {
    const chunk = makeEventChunk(defaultTimeInjector, src, events)
    return eventStore.persistEvents(chunk).map(c => c.map(ev => Event.toEnvelopeFromStore(ev)))
  }

  const jar = await hydrate(
    fish,
    testFishName,
    eventStore,
    snapshotStore,
    sendToStore,
    () => Observable.never(),
    CommandExecutor({}),
    mkNoopPondStateTracker(),
  ).toPromise()

  const observe = jar.publicSubject
  const pubEvents = eventStore.directlyPushEvents

  const applyAndGetState = async (events: Events, numExpectedStates = 1) => {
    // adding events may or may not emit a new state, depending on whether the events
    // were relevant (might be before semantic snapshot or duplicates)
    const pubProm = observe
      .take(1 + numExpectedStates)
      .timeout(100)
      .catch(() => Observable.empty())
      .toPromise()
    pubEvents(events)
    return pubProm
  }

  const latestSnap = async () =>
    snapshotStore
      .retrieveSnapshot(fish.semantics, testFishName, 1)
      .then(x => (x ? { ...x, state: JSON.parse(x.state) } : undefined))

  return {
    latestSnap,
    snapshotStore,
    applyAndGetState,
    observe,
    pubEvents,
    enqueueCommand: jar.enqueueCommand,
    dump: jar.dump,
  }
}

export const semanticSnap: SemanticSnapshot<NumberFishEvent> = () => env => env.payload === -1

export const localSnap = (
  version: number,
): SnapshotFormat<ReadonlyArray<number>, ReadonlyArray<number>> => SnapshotFormat.identity(version)

export type FishTestFn = (
  fish: FishTypeImpl<
    ReadonlyArray<number>,
    NumberFishEvent,
    NumberFishEvent,
    ReadonlyArray<number>
  >,
) => Promise<void>

export type TestFish = [
  string,
  FishTypeImpl<ReadonlyArray<number>, NumberFishEvent, NumberFishEvent, ReadonlyArray<number>>
]

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
