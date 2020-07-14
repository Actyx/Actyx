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
  mkNumberFish,
  mkSnapshot,
  mkTimeline,
  offsets,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'
import { Timestamp } from './types'

const numberFish = mkNumberFish(semanticSnap)

describe('fish event store + jar with both local and semantic snapshots', () => {
  it(`events below snapshot horizon should not shatter fish state`, async () => {
    const { mkEvents } = eventFactory()
    const { applyAndGetState, latestSnap } = await snapshotTestSetup(
      numberFish,
      [
        /* Intentionally omitting the events that would have formed the snapshot. */
      ],
      [mkSnapshot([8, 9, 10], 1000, 900)],
    )

    const belowHorizonEvents: Events = mkEvents([
      {
        timestamp: 100,
        source: 'B',
        payload: 6,
      },
      {
        timestamp: 200,
        source: 'B',
        payload: 7,
      },
    ])
    // Events below horizon have been ignored and our state kept intact.
    expect(await applyAndGetState(belowHorizonEvents)).toEqual([8, 9, 10])
    // Snapshot is still there.
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 1000 },
      horizon: { lamport: 900 },
      state: [8, 9, 10],
    })
  })

  it(`semantic snapshot events should invalidate all past local snapshots`, async () => {
    const { mkEvents } = eventFactory()
    const now = Timestamp.now()

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(
      numberFish,
      [],
      [mkSnapshot([8, 9, 10], now - 2000)],
    )

    const liveWithSemanticSnapshot: Events = mkEvents([
      {
        timestamp: now,
        source: 'B',
        payload: -1,
      },
      {
        timestamp: now + 100,
        source: 'B',
        payload: 1,
      },
    ])

    expect(await applyAndGetState(liveWithSemanticSnapshot)).toEqual([-1, 1])
    // No new snapshot yet since all known events are within 1h of latest known event.
    expect(await latestSnap()).toEqual(undefined)
  })

  it(`semantic snapshot events below snapshot horizon should not shatter fish state`, async () => {
    const { mkEvents } = eventFactory()
    const { applyAndGetState, latestSnap } = await snapshotTestSetup(
      numberFish,
      [],
      [mkSnapshot([8, 9, 10], 1000, 900)],
    )

    const belowHorizonEvents: Events = mkEvents([
      {
        timestamp: 100,
        source: 'B',
        payload: -1,
      },
    ])

    // Semantic snapshot event below horizon has been ignored and our state kept intact.
    expect(await applyAndGetState(belowHorizonEvents)).toEqual([8, 9, 10])
    // Snapshot is still there.
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 1000 },
      state: [8, 9, 10],
    })
  })

  it(`semantic snapshot events should shatter and timetravel`, async () => {
    const srcA = emitter('A')
    const srcB = emitter('B')

    const timeline = mkTimeline(srcA(1), srcB(-1), srcA(3), srcB(4), srcA(50))

    const oldEvents = timeline.of('A')

    const psns = offsets(oldEvents)

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish, oldEvents, [
      mkSnapshot([1, 3, 50], 5000, undefined, psns),
    ])

    const liveWithSemanticSnapshot: Events = timeline.of('B')

    const state = await applyAndGetState(liveWithSemanticSnapshot)
    expect(state).toEqual([-1, 3, 4, 50])
    expect(await latestSnap()).toEqual(undefined)
  })

  it(`should local-snapshot semantic snapshot events, eventually`, async () => {
    const srcA = emitter('A')
    const srcB = emitter('B')

    const timeline = mkTimeline(
      srcA(1),
      srcA(2),
      // Since the semantic snapshot isn't treated like a normal event,
      // it doesn't have *exactly* the same semantics for triggering snapshots --
      // the important thing is that it works.
      srcB(-1),
      srcB.triggerLocalSnapshot(),
      srcA(4),
      srcA.ageSnapshotsOverMinAge(),
    ).all

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

    expect(await applyAndGetState(timeline)).toEqual([-1, 4])
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 10240400 },
      state: [-1],
      horizon: { lamport: 400 },
    })
  })

  it(`semantic snapshot wedged between local snapshots`, async () => {
    const srcA = emitter('A')
    const srcB = emitter('B')

    const timeline = mkTimeline(
      srcA(1),
      srcA(2),
      srcB(3),
      srcB.triggerLocalSnapshot(),
      srcA(4),
      srcA.ageSnapshotsOverMinAge(),
      srcB(-1),
      srcB(5),
      srcB.triggerLocalSnapshot('large'),
      srcA(6),
      srcA.ageSnapshotsOverMinAge(),
    ).all

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

    expect(await applyAndGetState(timeline)).toEqual([-1, 5, 6])
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 7374080900 },
      state: [-1, 5],
      horizon: { lamport: 7210250800 },
    })
  })

  it(`semantic snapshot shattering down to local snapshot`, async () => {
    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')

    const timeline = mkTimeline(
      srcA(1),
      srcB(2),
      srcB(3),
      srcB.triggerLocalSnapshot(),
      srcA(4),
      srcA.ageSnapshotsOverMinAge(),
      srcB(5),
      srcC(6),
      srcC(-1),
      srcB(7),
      srcC(8),
    )

    const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

    expect(await applyAndGetState(timeline.of('A'))).toEqual([1, 4])
    expect(await latestSnap()).toBeUndefined()

    expect(await applyAndGetState(timeline.of('B'))).toEqual([1, 2, 3, 4, 5, 7])
    expect(await latestSnap()).toMatchObject({
      eventKey: { lamport: 10210400 },
      state: [1, 2, 3],
      horizon: undefined,
    })

    expect(await applyAndGetState(timeline.of('C'))).toEqual([-1, 7, 8])
    expect(await latestSnap()).toBeUndefined() // Removed by the semantic snapshot.
  })

  describe(`horizon`, () => {
    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')

    const timeline = mkTimeline(
      srcA(1),
      srcC(2),
      srcA(-1),
      srcA(3),
      srcA.triggerLocalSnapshot(),
      srcA(4),
      srcA.ageSnapshotsOverMinAge(),
      srcC(5),
      srcB(6),
      srcB.triggerLocalSnapshot('medium'),
      srcB(7),
      srcB.ageSnapshotsOverMinAge(),
      srcC(8),
    )

    const expectedFinalState = [-1, 3, 4, 5, 6, 7, 8]
    const expectedFinalSnap = {
      eventKey: { lamport: 7251161000 },
      state: [-1, 3, 4, 5, 6],
      horizon: { lamport: 400 },
    }

    it(`should be preserved from semantic snapshots via local snapshots`, async () => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

      expect(await applyAndGetState(timeline.of('A', 'C'))).toEqual([-1, 3, 4, 5, 8])
      expect(await latestSnap()).toMatchObject({
        eventKey: { lamport: 10230500 },
        state: [-1, 3],
        horizon: { lamport: 400 },
      })

      expect(await applyAndGetState(timeline.of('B'))).toEqual(expectedFinalState)
      expect(await latestSnap()).toMatchObject(expectedFinalSnap)
    })

    it(`should be respected when shattering, by not getting the low events`, async () => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

      expect(await applyAndGetState(timeline.of('A', 'B'))).toEqual([-1, 3, 4, 6, 7])
      expect(await latestSnap()).toMatchObject({
        eventKey: { lamport: 7251171000 },
        state: [-1, 3, 4, 6],
        horizon: { lamport: 400 },
      })

      expect(await applyAndGetState(timeline.of('C'))).toEqual(expectedFinalState)
      expect(await latestSnap()).toMatchObject(expectedFinalSnap)
    })

    it(`should be retroactively found when timetravelling`, async () => {
      const { applyAndGetState, latestSnap } = await snapshotTestSetup(numberFish)

      expect(await applyAndGetState(timeline.of('B', 'C'))).toEqual([2, 5, 6, 7, 8])
      expect(await latestSnap()).toMatchObject({
        eventKey: { lamport: 7251181000 },
        state: [2, 5, 6],
        horizon: undefined,
      })

      expect(await applyAndGetState(timeline.of('A'))).toEqual(expectedFinalState)
      expect(await latestSnap()).toMatchObject(expectedFinalSnap)
    })
  })
})
