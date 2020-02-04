import { Events } from './eventstore/types'
import {
  eventFactory,
  forFishes,
  localSnap,
  mkNumberFish,
  semanticSnap,
  snapshotTestSetup,
} from './fish.testHelper'
import { Subscription } from './subscription'

/* Fish tests that do not explicitly rely on snapshots.
 * We still test fishes with all possible snapshot config configurations,
 * in order to make sure that basic funcionality is not screwed
 * by specialized snapshot logic. */

const noSnapshotsFish = mkNumberFish((semantics, name) => [Subscription.of(semantics, name)])

const semanticSnapshotsFish = mkNumberFish(
  (semantics, name) => [Subscription.of(semantics, name)],
  semanticSnap,
)

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

const forAllFish = forFishes(
  ['without snapshots', noSnapshotsFish],
  ['with only semantic snapshots', semanticSnapshotsFish],
  ['with only local snapshots', localSnapshotsFish],
  ['with all types of snapshots', allSnapshotsFish],
)

describe('fish event store + jar snapshot agnostic behaviour', () => {
  const { mkEvents } = eventFactory()
  const aEvents: Events = mkEvents([
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

  const bEvents: Events = mkEvents([
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
    const { applyAndGetState } = await snapshotTestSetup(fish)

    expect(await applyAndGetState(aEvents)).toEqual([1, 3])

    expect(await applyAndGetState(bEvents)).toEqual([1, 2, 3, 4])
  })

  forAllFish(`should deal properly with unsorted live chunks, A first`, async fish => {
    const { applyAndGetState } = await snapshotTestSetup(fish)

    expect(await applyAndGetState(aEvents.concat(bEvents), 2)).toEqual([1, 2, 3, 4])
  })

  forAllFish(`should deal properly with unsorted live chunks, B first`, async fish => {
    const { applyAndGetState } = await snapshotTestSetup(fish)

    expect(await applyAndGetState(bEvents.concat(aEvents), 2)).toEqual([1, 2, 3, 4])
  })
})
