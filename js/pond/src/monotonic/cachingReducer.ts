import { last, partition } from 'fp-ts/lib/Array'
import { Option } from 'fp-ts/lib/Option'
import { gt } from 'fp-ts/lib/Ord'
import log from '../loggers'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { EventKey, LocalSnapshot, Timestamp } from '../types'
import { CachingReducer, PendingSnapshot, SerializedStateSnap, SimpleReducer } from './types'

const eventKeyGreater = gt(EventKey.ord)

// Wrap a normal, very simple Reducer into one that caches intermediate states
// and schedules persistence as local snapshots.
export const cachingReducer = <S>(
  simpleReducer: SimpleReducer<S>,
  snapshotScheduler: SnapshotScheduler,
  storeSnapshot: (toStore: PendingSnapshot) => Promise<void>,
  deserializeState?: (jsonState: unknown) => S,
): CachingReducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

  const deserializeSnapshot = (snap: SerializedStateSnap): LocalSnapshot<S> => {
    const snapState = deserialize(snap.state)
    return { ...snap, state: snapState }
  }

  const queue = snapshotQueue()

  // Chain of snapshot storage promises
  let storeSnapshotsPromise: Promise<void> = Promise.resolve()

  const snapshotEligible = (latest: Timestamp) => (snapBase: PendingSnapshot) =>
    snapshotScheduler.isEligibleForStorage(snapBase, { timestamp: latest })

  // This is needed for scheduling local snapshots. It counts events since last semantic snapshot.
  let cycle = 0

  const appendEvents: CachingReducer<S>['appendEvents'] = events => {
    // FIXME: Arguments are a bit questionable, but we can’t change the scheduler yet, otherwise the FES-based tests start failing.
    const statesToStore = snapshotScheduler.getSnapshotLevels(cycle + 1, events, 0)

    let fromIdx = 0
    for (const toStore of statesToStore) {
      const stateWithProvenance = simpleReducer.appendEvents(events, fromIdx, toStore.i)
      fromIdx = toStore.i + 1

      queue.addPending({
        snap: {
          ...stateWithProvenance,
          state: JSON.stringify(stateWithProvenance.state),
        },
        tag: toStore.tag,
        timestamp: events[toStore.i].timestamp,
      })
    }

    const latestState = simpleReducer.appendEvents(events, fromIdx, events.length - 1)
    cycle = latestState.cycle

    const snapshotsToPersist =
      events.length > 0
        ? queue.getSnapshotsToStore(snapshotEligible(events[events.length - 1].timestamp))
        : []

    if (snapshotsToPersist.length > 0) {
      storeSnapshotsPromise = storeSnapshotsPromise.then(() =>
        Promise.all(snapshotsToPersist.map(storeSnapshot))
          .then(() => undefined)
          .catch(log.pond.warn),
      )
    }

    return latestState
  }

  const setState = (snap: SerializedStateSnap) => {
    // Time travel to the past: All newer cached states are invalid
    queue.invalidateLaterThan(snap.eventKey)

    cycle = snap.cycle

    simpleReducer.setState(deserializeSnapshot(snap))
  }

  return {
    appendEvents,
    awaitPendingPersistence: () => storeSnapshotsPromise,
    latestKnownValidState: (lowestInvalidating, highestInvalidating) =>
      queue.latestValid(lowestInvalidating, highestInvalidating).map(x => x.snap),
    setState,
  }
}

type SnapshotQueue = {
  // Add a pending snapshot
  addPending: (snap: PendingSnapshot) => void

  // Evict all cached states based on events after the cutoff.
  invalidateLaterThan: (cutOff: EventKey) => void

  // Pop all snapshots from the queue which should be stored now, according to the given predicate
  getSnapshotsToStore: (
    storeNow: (snapshot: PendingSnapshot) => boolean,
  ) => ReadonlyArray<PendingSnapshot>

  // Try to find the latest still valid snapshot according to the given arguments.
  // Only calling this function does not evict anything
  latestValid: (
    lowestInvalidating: EventKey,
    highestInvalidating: EventKey,
  ) => Option<PendingSnapshot>
}

const snapshotQueue = (): SnapshotQueue => {
  let queue: PendingSnapshot[] = []

  const addPending = (snap: PendingSnapshot) => {
    queue.push(snap)
  }

  const invalidateLaterThan = (cutOff: EventKey) => {
    queue = queue.filter(entry => eventKeyGreater(cutOff, entry.snap.eventKey))
  }

  const getSnapshotsToStore = (
    storeNow: (snapshot: PendingSnapshot) => boolean,
  ): ReadonlyArray<PendingSnapshot> => {
    const seperated = partition(queue, storeNow)

    queue = seperated.left

    return seperated.right
  }

  const latestValid = (
    lowestInvalidating: EventKey,
    highestInvalidating: EventKey,
  ): Option<PendingSnapshot> => {
    return last(
      queue.filter(({ snap }) => {
        const snapshotValid = eventKeyGreater(lowestInvalidating, snap.eventKey)
        const horizonVoidsTimeTravel =
          snap.horizon && eventKeyGreater(snap.horizon, highestInvalidating)
        return snapshotValid || horizonVoidsTimeTravel
      }),
    )
  }

  return {
    addPending,
    invalidateLaterThan,
    latestValid,
    getSnapshotsToStore,
  }
}
