/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Events } from './eventstore/types'
import {
  emitter,
  eventFactory,
  forFishes,
  localSnap,
  mkNumberFish,
  mkSnapshot,
  mkTimeline,
  offsets,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'
import { Subscription } from './subscription'

const localSnapshotsFish = mkNumberFish(
  (semantics, name) => [Subscription.of(semantics, name)],
  undefined,
  localSnap(1),
)

const allSnapshotsFish = mkNumberFish(
  (semantics, name) => [Subscription.of(semantics, name)],
  semanticSnap,
  localSnap(1),
)

const forBoth = forFishes(
  ['with only local snapshots', localSnapshotsFish],
  ['with all types of snapshots', allSnapshotsFish],
)

describe('fish event store + jar local snapshot behavior', () => {
  forBoth(
    `should create local snapshot for after seeing that enough time has passed from live event`,
    async fishToTest => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest)

      const srcV = emitter('V')
      const srcR = emitter('R')
      const srcQ = emitter('Q')

      const tl = mkTimeline(
        srcR(5),
        srcR(6),
        srcR.triggerLocalSnapshot(),
        srcV(4),
        srcR(7),
        srcQ(8),
        srcR(9),
        srcR.ageSnapshotsOverMinAge(),
      )

      const rqTime = tl.of('R', 'Q')
      const cutoff = rqTime.findIndex(e => e.payload === 8) + 1

      const oldEvents = tl.of('R', 'Q').slice(0, cutoff)
      expect(await applyAndGetState(oldEvents)).toEqual([5, 6, 7, 8])
      expect(await latestSnap()).toBeUndefined()

      const newEvents = tl.of('R', 'Q').slice(cutoff)
      expect(await applyAndGetState(newEvents)).toEqual([5, 6, 7, 8, 9])

      expect(await applyAndGetState(tl.of('V'))).toEqual([5, 6, 4, 7, 8, 9])

      expect(await latestSnap()).toMatchObject({
        eventKey: { lamport: 10220300 },
        state: [5, 6],
      })
    },
  )

  forBoth(`should create local snapshot wholly from live events`, async fishToTest => {
    const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest)

    const srcR = emitter('R')

    const timeline = mkTimeline(
      srcR(5),
      srcR(6),
      srcR(7),
      srcR.triggerLocalSnapshot(),
      srcR(8),
      srcR.ageSnapshotsOverMinAge(),
    ).all

    expect(await applyAndGetState(timeline)).toEqual([5, 6, 7, 8])
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 10210400 },
      state: [5, 6, 7],
    })
  })

  forBoth(`should create local snapshot during hydration`, async fishToTest => {
    const srcR = emitter('R')

    const timeline = mkTimeline(
      srcR(5),
      srcR(6),
      srcR(7),
      srcR.triggerLocalSnapshot('medium'),
      srcR.ageSnapshotsOverMinAge(),
      srcR(8),
      srcR(9),
    ).all

    const storedEvents = timeline.slice(0, 4)

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest, storedEvents)

    // Need to emit sth. in order to observe
    const liveEvents = timeline.slice(4)

    expect(await applyAndGetState(liveEvents)).toEqual([5, 6, 7, 8, 9])
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 40930400 },
      state: [5, 6, 7],
    })
  })

  describe(`should create multiple local snapshots`, () => {
    const srcR = emitter('R')
    const srcQ = emitter('Q')

    const timeline = mkTimeline(
      srcR(5),
      srcQ(6),
      srcR(7),
      srcR.triggerLocalSnapshot('medium'),
      srcR(8),
      srcR.triggerLocalSnapshot('large'),
      srcR.ageSnapshotsOverMinAge(),
      srcQ(9),
      srcQ(10),
    )

    const expectedSnap = {
      eventKey: { lamport: 204770500 },
      state: [5, 6, 7, 8],
    }

    forBoth(`when ingesting all sources at once`, async fishToTest => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest)

      expect(await applyAndGetState(timeline.all)).toEqual([5, 6, 7, 8, 9, 10])
      expect(await latestSnap()).toMatchObject(expectedSnap)
    })

    forBoth(`when seeing R first`, async fishToTest => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest)

      expect(await applyAndGetState(timeline.of('R'))).toEqual([5, 7, 8])
      expect(await applyAndGetState(timeline.of('Q'))).toEqual([5, 6, 7, 8, 9, 10])
      expect(await latestSnap()).toMatchObject(expectedSnap)
    })

    forBoth(`when seeing Q first`, async fishToTest => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(fishToTest)

      expect(await applyAndGetState(timeline.of('Q'))).toEqual([6, 9, 10])
      expect(await applyAndGetState(timeline.of('R'))).toEqual([5, 6, 7, 8, 9, 10])
      expect(await latestSnap()).toMatchObject(expectedSnap)
    })
  })

  forBoth(`should hydrate from local snapshot`, async fishToTest => {
    const { mkEvents } = eventFactory()
    const storedEvents: Events = [
      // We intentionally leave out the events that would have formed the snapshot,
      // in order to assert that the snapshot is really used for hydration
    ]
    const storedSnaps = [mkSnapshot([8, 9, 10], 500)]

    const { applyAndGetState } = await snapshotTestSetup(fishToTest, storedEvents, storedSnaps)

    const currentEvents: Events = mkEvents([
      {
        timestamp: 600,
        source: 'A',
        payload: 20,
      },
    ])
    // Hydrated from snapshot and appended '20'
    expect(await applyAndGetState(currentEvents)).toEqual([8, 9, 10, 20])
  })

  forBoth(`should shatter local snapshot if it receives earlier live events`, async fishToTest => {
    const srcA = emitter('A')
    const srcB = emitter('B')

    const timeline = mkTimeline(srcA(1), srcB(2), srcA(3), srcA(4), srcB(5))

    const storedEvents = timeline.of('A')
    const storedSnaps = [mkSnapshot([1, 3, 4], 40000, undefined, offsets(storedEvents))]

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(
      fishToTest,
      storedEvents,
      storedSnaps,
    )
    // Make sure it did not shatter yet, because the stored events are covered by its psn map.
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 40000 },
      state: [1, 3, 4],
    })

    const pastEvents: Events = timeline.of('B')
    expect(await applyAndGetState(pastEvents)).toEqual([1, 2, 3, 4, 5])
    expect(await latestSnap()).toEqual(undefined)
  })

  forBoth(
    `fish should shatter local snapshot if it receives earlier stored events`,
    async fishToTest => {
      const a = emitter('a')
      const b = emitter('b')
      const c = emitter('c')
      // We will omit d from the snapshot, ie. it is a yet unknown source.
      const d = emitter('d')

      const tl = mkTimeline(a(1), a(2), d(3), b(4), c(5), a(6), c(7))

      const knownEvents = [tl.of('a'), tl.of('b'), tl.of('c')]
      const snapshotOffsets = offsets(...knownEvents)
      // const storedEvents = flatten(knownEvents)

      const storedSnaps = [mkSnapshot([1, 2, 4, 5, 6, 7], 200000, undefined, snapshotOffsets)]

      const { applyAndGetState, latestSnap } = await snapshotTestSetup(
        fishToTest,
        tl.all,
        storedSnaps,
      )
      // Assert the snapshot has already been invalidated in the initial hydration.
      expect(await latestSnap()).toEqual(undefined)

      // Some later event, just because we need to feed something in order for observe to emit.
      const liveEvent = eventFactory().mkEvent({
        payload: 3000,
        timestamp: Number.MAX_SAFE_INTEGER,
        source: 'foo',
      })
      // Assert shatter and timetravel.
      expect(await applyAndGetState([liveEvent])).toEqual([1, 2, 3, 4, 5, 6, 7, 3000])
    },
  )
})
