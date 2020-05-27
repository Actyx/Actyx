/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Events } from './eventstore/types'
import {
  emitter,
  forFishes,
  localSnap,
  mkNumberFish,
  mkTimeline,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'
import { Subscription } from './subscription'

const semanticSnapshotsFish = mkNumberFish(
  (semantics, name) => [Subscription.of(semantics, name)],
  semanticSnap,
)

const allSnapshotsFish = mkNumberFish(
  (semantics, name) => [Subscription.of(semantics, name)],
  semanticSnap,
  localSnap(1),
)

const forBoth = forFishes(
  ['with only semantic snapshots', semanticSnapshotsFish],
  ['with all types of snapshots', allSnapshotsFish],
)

describe('fish event store + jar semantic snapshot functionality', () => {
  forBoth(
    `fish should aggegrate events between sources, resetting on semantic snapshots`,
    async fish => {
      const { applyAndGetState } = await snapshotTestSetup(fish)

      const srcA = emitter('A')
      const srcB = emitter('B')
      const srcC = emitter('C')
      const tl = mkTimeline(srcA(3), srcA(7), srcB(-1), srcC(8))

      expect(await applyAndGetState(tl.of('A'))).toEqual([3, 7])
      expect(await applyAndGetState(tl.of('B', 'C'))).toEqual([-1, 8])
    },
  )

  forBoth(`fish should reset on semantic snapshot inside chunk`, async fish => {
    const { applyAndGetState } = await snapshotTestSetup(fish)

    const srcA = emitter('A')
    const events: Events = mkTimeline(srcA(3), srcA(7), srcA(-1), srcA(8)).all
    expect(await applyAndGetState(events)).toEqual([-1, 8])
  })

  describe(`reset behaviour`, () => {
    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')
    const tl = mkTimeline(
      srcA(3),
      srcA(4),
      srcB(5),
      srcA(7),
      srcB(-1),
      srcC(8),
      srcC(-1),
      srcC(9),
      srcB(10),
      srcA(11),
    )

    forBoth(`should reset with every new latest semantic snapshot`, async fish => {
      const { applyAndGetState } = await snapshotTestSetup(fish)

      expect(await applyAndGetState(tl.of('A'))).toEqual([3, 4, 7, 11])
      expect(await applyAndGetState(tl.of('B'))).toEqual([-1, 10, 11])
      expect(await applyAndGetState(tl.of('C'))).toEqual([-1, 9, 10, 11])
    })

    forBoth(`should ignore semantic snapshots older than current latest`, async fish => {
      const { applyAndGetState } = await snapshotTestSetup(fish)

      expect(await applyAndGetState(tl.of('A'))).toEqual([3, 4, 7, 11])
      expect(await applyAndGetState(tl.of('C'))).toEqual([-1, 9, 11])
      expect(await applyAndGetState(tl.of('B'))).toEqual([-1, 9, 10, 11])
    })

    forBoth(`should late comer source nicely`, async fish => {
      const { applyAndGetState } = await snapshotTestSetup(fish)

      expect(await applyAndGetState(tl.of('B', 'C'))).toEqual([-1, 9, 10])
      expect(await applyAndGetState(tl.of('A'))).toEqual([-1, 9, 10, 11])
    })
  })
})
