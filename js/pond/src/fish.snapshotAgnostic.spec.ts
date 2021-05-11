/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import {
  eventFactory,
  forFishes,
  mkNumberFish,
  NumberFishEvent,
  NumberFishState,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'
import { Fish } from './types'

/* Fish tests that do not explicitly rely on snapshots.
 * We still test fishes with all possible snapshot config configurations,
 * in order to make sure that basic funcionality is not screwed
 * by specialized snapshot logic. */

const semanticSnapshotsFish = mkNumberFish(semanticSnap)

const localSnapshotsFish = mkNumberFish()

const forAllFish = forFishes(
  ['with semantic snapshots', semanticSnapshotsFish],
  ['with only local snapshots', localSnapshotsFish],
)

const setup = (fish: Fish<NumberFishState, NumberFishEvent>) => snapshotTestSetup(fish)

describe('fish event store + jar snapshot agnostic behaviour', () => {
  const { mkEvents } = eventFactory()
  const aEvents = mkEvents([
    {
      timestamp: 100,
      source: 'A',
      payload: 1,
    },
    {
      timestamp: 300,
      source: 'A',
      payload: 3,
    },
  ])

  const bEvents = mkEvents([
    {
      timestamp: 200,
      source: 'B',
      payload: 2,
    },
    {
      timestamp: 400,
      source: 'B',
      payload: 4,
    },
  ])

  forAllFish(`should put events into the right order for state computation`, async fish => {
    const { applyAndGetState } = await setup(fish)

    expect(await applyAndGetState(aEvents)).toEqual([1, 3])

    expect(await applyAndGetState(bEvents)).toEqual([1, 2, 3, 4])
  })

  forAllFish(`should deal properly with unsorted live chunks, A first`, async fish => {
    const { applyAndGetState } = await setup(fish)

    expect(await applyAndGetState(aEvents.concat(bEvents))).toEqual([1, 2, 3, 4])
  })

  forAllFish(`should deal properly with unsorted live chunks, B first`, async fish => {
    const { applyAndGetState } = await setup(fish)

    expect(await applyAndGetState(bEvents.concat(aEvents))).toEqual([1, 2, 3, 4])
  })
})
