import { EventKey, Timestamp } from '@actyx/sdk'
import { last, partition } from 'fp-ts/lib/Array'
import { none, Option, some } from 'fp-ts/lib/Option'
import { gt } from 'fp-ts/lib/Ord'
import log from '../loggers'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { LocalSnapshot } from '../types'
import { CachingReducer, PendingSnapshot, SerializedStateSnap, SimpleReducer } from './types'

const eventKeyGreater = gt(EventKey.ord)

// Wrap a normal, very simple Reducer into one that caches intermediate states
// and schedules persistence as local snapshots.
export const cachingReducer = <S>(
  simpleReducer: SimpleReducer<S>,
  snapshotScheduler: SnapshotScheduler,
  storeSnapshot: (toStore: PendingSnapshot) => Promise<void>,
  deserializeState: undefined | ((jsonState: unknown) => S),
): CachingReducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

  const deserializeSnapshot = (snap: SerializedStateSnap): LocalSnapshot<S> => {
    const snapState = deserialize(snap.state)
    return { ...snap, state: snapState }
  }

  const serialize = (original: LocalSnapshot<S>): SerializedStateSnap => ({
    ...original,
    state: JSON.stringify(original.state),
  })

  const queue = snapshotQueue()

  // Chain of snapshot storage promises
  let storeSnapshotsPromise: Promise<void> = Promise.resolve()

  const snapshotEligible = (latest: Timestamp) => (snapBase: PendingSnapshot) =>
    snapshotScheduler.isEligibleForStorage(snapBase, { timestamp: latest })

  let latestStateSerialized: Option<SerializedStateSnap> = none

  const appendEvents: CachingReducer<S>['appendEvents'] = events => {
    // FIXME: Arguments are a bit questionable, but we can’t change the scheduler yet, otherwise the FES-based tests start failing.
    const statesToStore = snapshotScheduler.getSnapshotLevels(
      latestStateSerialized.map(x => x.cycle).getOrElse(0),
      events,
      0,
    )

    let fromIdx = 0
    for (const toStore of statesToStore) {
      const headState = simpleReducer.appendEvents(events, fromIdx, toStore.i)
      fromIdx = toStore.i + 1

      queue.addPending({
        snap: serialize(headState),
        tag: toStore.tag,
        timestamp: events[toStore.i].timestamp,
      })
    }

    const latestState = simpleReducer.appendEvents(events, fromIdx, events.length - 1)

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

    // Make another copy so that downstream doesn’t mutate what’s in the SimpleReducer
    const latestStateSer = serialize(latestState)
    latestStateSerialized = some(latestStateSer)
    return deserializeSnapshot(latestStateSer)
  }

  const setState = (snap: SerializedStateSnap) => {
    // Time travel to the past: All newer cached states are invalid
    queue.invalidateLaterThan(snap.eventKey)

    latestStateSerialized = some(snap)
    simpleReducer.setState(deserializeSnapshot(snap))
  }

  return {
    appendEvents,
    awaitPendingPersistence: () => storeSnapshotsPromise,
    latestKnownValidState: (lowestInvalidating, highestInvalidating) =>
      latestStateSerialized
        .filter(isSnapshotValid(lowestInvalidating, highestInvalidating))
        .orElse(() => queue.latestValid(lowestInvalidating, highestInvalidating).map(x => x.snap)),
    setState,
  }
}

const isSnapshotValid = (lowestInvalidating: EventKey, highestInvalidating: EventKey) => (
  snap: SerializedStateSnap,
): boolean => {
  const snapshotValid = eventKeyGreater(lowestInvalidating, snap.eventKey)
  const horizonVoidsTimeTravel =
    !!snap.horizon && eventKeyGreater(snap.horizon, highestInvalidating)
  return snapshotValid || horizonVoidsTimeTravel
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
      queue.filter(({ snap }) => isSnapshotValid(lowestInvalidating, highestInvalidating)(snap)),
    )
  }

  return {
    addPending,
    invalidateLaterThan,
    latestValid,
    getSnapshotsToStore,
  }
}
