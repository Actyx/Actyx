/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Events } from './eventstore/types'
import {
  emitter,
  mkNumberFish,
  mkTimeline,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'

const fish = mkNumberFish(semanticSnap)

const setup = () => snapshotTestSetup(fish, undefined, undefined, true)

describe('fish event store + jar semantic snapshot functionality', () => {
  it(`fish should aggegrate events between sources, resetting on semantic snapshots`, async () => {
    const { applyAndGetState } = await setup()

    const srcA = emitter('A')
    const srcB = emitter('B')
    const srcC = emitter('C')
    const tl = mkTimeline(srcA(3), srcA(7), srcB(-1), srcC(8))

    expect(await applyAndGetState(tl.of('A'))).toEqual([3, 7])
    expect(await applyAndGetState(tl.of('B', 'C'))).toEqual([-1, 8])
  })

  it(`fish should reset on semantic snapshot inside chunk`, async () => {
    const { applyAndGetState } = await setup()

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

    it(`should reset with every new latest semantic snapshot`, async () => {
      const { applyAndGetState } = await setup()

      expect(await applyAndGetState(tl.of('A'))).toEqual([3, 4, 7, 11])
      expect(await applyAndGetState(tl.of('B'))).toEqual([-1, 10, 11])
      expect(await applyAndGetState(tl.of('C'))).toEqual([-1, 9, 10, 11])
    })

    it(`should ignore semantic snapshots older than current latest`, async () => {
      const { applyAndGetState } = await setup()

      expect(await applyAndGetState(tl.of('A'))).toEqual([3, 4, 7, 11])
      expect(await applyAndGetState(tl.of('C'))).toEqual([-1, 9, 11])
      expect(await applyAndGetState(tl.of('B'))).toEqual([-1, 9, 10, 11])
    })

    it(`should late comer source nicely`, async () => {
      const { applyAndGetState } = await setup()

      expect(await applyAndGetState(tl.of('B', 'C'))).toEqual([-1, 9, 10])
      expect(await applyAndGetState(tl.of('A'))).toEqual([-1, 9, 10, 11])
    })
  })
})
