import { OffsetMap } from '../eventstore'
import { MultiplexedWebsocket } from '../eventstore/multiplexedWebsocket'
import { EventKey, FishName, LocalSnapshot, Semantics } from '../types'
import { InMemSnapshotStore } from './inMemSnapshotStore'
import { WebsocketSnapshotStore } from './websocketSnapshotStore'

export type LocalSnapshotFromIndex = LocalSnapshot<unknown>

export type StoreSnapshot = (
  semantics: Semantics,
  name: FishName,
  key: EventKey,
  psnMap: OffsetMap,
  horizon: EventKey | undefined,
  cycle: number,
  version: number,
  tag: string,
  blob: unknown,
) => Promise<boolean>

export type RetrieveSnapshot = (
  semantics: Semantics,
  name: FishName,
  version: number,
) => Promise<LocalSnapshotFromIndex | undefined>

export type InvalidateSnapshots = (
  semantics: Semantics,
  name: FishName,
  key: EventKey,
) => Promise<void>

export type InvalidateAllSnapshots = () => Promise<void>

/**
 * Interface to the snapshot store. This is colocated with the ipfs index store, but completely independent
 * conceptually.
 */
export interface SnapshotStore {
  /**
   * Store local snapshot (best effort)
   *
   * @param version For each semantics, the store is partitioned into versions corresponding to the snapshot format.
   * Only the newest known version should be kept.
   *
   * @param key is the EventKey of the event from which the snapshot state was computed.
   *
   * @param tag is a unique identifier for a given semantics, name, and version; it is used to ensure that only
   * one snapshot is kept for the a given interval (hour, day, month, year)
   *
   * @param psnMap is needed to recognize whether the given snapshot has already been invalidated by the
   * root updates that have been performed between the one that triggered the snapshot computation and now
   *
   * @returns success if the snapshot was accepted, false if it was rejected due to using an old format
   *
   * A snapshot is out of date if between the event state it represents (as demonstrated by the given psnMap)
   * and the current index store state there have been root updates that would have invalidated the snapshot.
   *
   * Snapshots will not be validated against events on storage. The only validation that might happen is that
   * the format version is the latest.
   */
  storeSnapshot: StoreSnapshot

  /**
   * Retrieve local snapshot and its EventKey if it exists; undefined otherwise.
   * The method also returns the PsnMap from which the snapshot was calculated.
   *
   * Most of the time, snapshots returned by this method will be valid, meaning that all stored events with
   * larger psns than the snapshot also have larger event keys that the snapshot.
   *
   * However, this is not guaranteed. In very rare circumstances it is possible that there are stored events with
   * a psn above the snapshot psnMap, but an eventKey below the snapshot eventKey. In these cases, the snapshot
   * needs to be discarded, exactly as if there was a realtime event coming in with a smaller eventKey than
   * the snapshot.
   *
   * To be precise, invalid snapshots will happen only if the application crashes between the time events are added
   * at the store level (e.g. to the recvlog) and the time the events appear in the fishjar, triggering explicit
   * invalidation. This should be very rare, but still needs to be handled properly.
   */
  retrieveSnapshot: RetrieveSnapshot

  /**
   * Invalidate all snapshots for the fish identified by semantics and name that are at or above the given event key.
   *
   * @param semantics semantics of the fish to invalidate snapshots for
   * @param name name of the fish to invalidate snapshots for
   * @param eventKey eventKey at or above which to purge snapshots
   * @returns a void promise that will complete once the snapshot invalidation is done in the persistence layer.
   */
  invalidateSnapshots: InvalidateSnapshots

  /**
   * Invalidate all snapshots
   *
   * @returns a void promise that will complete once the snapshot invalidation is done in the persistence layer.
   */
  invalidateAllSnapshots: InvalidateAllSnapshots
}

const noopSnapshotStore: SnapshotStore = {
  storeSnapshot: () => Promise.resolve(false),
  retrieveSnapshot: () => Promise.resolve(undefined),
  invalidateSnapshots: () => Promise.resolve(undefined),
  invalidateAllSnapshots: () => Promise.resolve(undefined),
}

export const SnapshotStore = {
  noop: noopSnapshotStore,
  inMem: InMemSnapshotStore.of,
  ws: (multiplexedWebsocket: MultiplexedWebsocket) =>
    new WebsocketSnapshotStore(multiplexedWebsocket),
}
