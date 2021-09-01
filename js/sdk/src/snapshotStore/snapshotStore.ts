/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { Observable } from '../../node_modules/rxjs'
import { EventKey, Lamport, LocalSnapshot, Offset, OffsetMap, StreamId } from '../types'

export interface Multiplexer {
  request: (reqType: string, payload?: unknown) => Observable<unknown>
}

export type LocalSnapshotFromIndex = LocalSnapshot<string>

export type StoreSnapshot = (
  semantics: string,
  name: string,
  key: EventKey,
  offsets: OffsetMap,
  horizon: EventKey | undefined,
  cycle: number,
  version: number,
  tag: string,
  serializedBlob: string,
) => Promise<boolean>

export type RetrieveSnapshot = (
  semantics: string,
  name: string,
  version: number,
) => Promise<LocalSnapshotFromIndex | undefined>

export type InvalidateSnapshots = (semantics: string, name: string, key: EventKey) => Promise<void>

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

const later: <T>(block: () => T) => Promise<T> = block => Promise.resolve().then(block)

type SnapshotKey = Readonly<{
  semantics: string
  name: string
  version: number
  tag: string
}>

type SnapshotRow = SnapshotKey &
  Readonly<{
    lamport: Lamport
    stream: StreamId
    offset: Offset
    blob: string
    rootsPsn: OffsetMap
    horizon: EventKey | undefined
    cycle: number
  }>

const selectSnapshotKey = (r: SnapshotRow): SnapshotKey => ({
  semantics: r.semantics,
  name: r.name,
  version: r.version,
  tag: r.tag,
})

const toSnapshotRow = (
  semantics: string,
  name: string,
  key: EventKey,
  version: number,
  tag: string,
  blob: string,
  offsets: OffsetMap,
  horizon: EventKey | undefined,
  cycle: number,
): SnapshotRow => ({
  semantics,
  name,
  lamport: key.lamport,
  stream: key.stream,
  offset: key.offset,
  version,
  tag,
  blob,
  rootsPsn: offsets,
  horizon,
  cycle,
})

type SnapshotMap = {
  [key: string]: SnapshotRow
}

class Impl implements SnapshotStore {
  private snapshots: SnapshotMap = {}

  storeSnapshot: StoreSnapshot = (
    sem: string,
    name: string,
    key: EventKey,
    psnMap: OffsetMap,
    horizon: EventKey | undefined,
    cycle: number,
    version: number,
    tag: string,
    blob: string,
  ) => {
    const snapshotRow = toSnapshotRow(sem, name, key, version, tag, blob, psnMap, horizon, cycle)
    const snapshotKey = selectSnapshotKey(snapshotRow)
    return later(() => {
      const values = Object.values(this.snapshots)
      const higherVersionValues = values.filter(
        sr => sr.semantics === sem && sr.name === name && sr.version > version,
      )
      // fail if attempting to add a snapshot with version smaller than currently existing
      if (higherVersionValues.length > 0) {
        return false
      }
      // now remove (strictly) older versions
      const newValues = values.filter(
        sr => !(sr.semantics === sem && sr.name === name && sr.version < version),
      )
      this.snapshots = newValues.reduce<SnapshotMap>(
        (acc, value) => ({ ...acc, [JSON.stringify(selectSnapshotKey(value))]: value }),
        {},
      )

      // and push the current snapshot (perhaps overwriting the one with the same (semantics, name, version, tag))
      this.snapshots[JSON.stringify(snapshotKey)] = snapshotRow
      return true
    })
  }

  retrieveSnapshot: RetrieveSnapshot = (s: string, n: string, v: number) => {
    const reverseKeyOrder = (l: SnapshotRow, r: SnapshotRow) => EventKey.ord.compare(r, l)
    const values = Object.values(this.snapshots)
    const retrievedSnapshots = values
      .filter(
        snapshotRow =>
          snapshotRow.semantics === s && snapshotRow.name === n && snapshotRow.version === v,
      )
      .sort(reverseKeyOrder)
    return later<LocalSnapshotFromIndex | undefined>(() => {
      if (retrievedSnapshots.length === 0) {
        return undefined
      } else {
        const snap = retrievedSnapshots[0]
        const eventKey: EventKey = {
          lamport: snap.lamport,
          stream: snap.stream,
          offset: snap.offset,
        }
        const offsets: OffsetMap = snap.rootsPsn
        // psnMap is readonly in production.
        Object.preventExtensions(offsets)
        const horizon = snap.horizon
        const cycle = snap.cycle
        return { eventKey, state: snap.blob, offsets, horizon, cycle }
      }
    })
  }

  invalidateSnapshots: InvalidateSnapshots = (sem: string, name: string, key: EventKey) => {
    return later(() => {
      const snapshotAfterOrOnEventKey = (
        lamport: Lamport,
        stream: StreamId,
        offset: Offset,
        eventKey: EventKey,
      ): boolean => {
        const snapshotEventKey: EventKey = { lamport, stream, offset }
        return EventKey.ord.compare(snapshotEventKey, eventKey) >= 0
      }
      const values = Object.values(this.snapshots)
      const newValues = values.filter(
        sr =>
          !(
            sr.semantics === sem &&
            sr.name === name &&
            snapshotAfterOrOnEventKey(sr.lamport, sr.stream, sr.offset, key)
          ),
      )

      this.snapshots = newValues.reduce<SnapshotMap>(
        (acc, value) => ({ ...acc, [JSON.stringify(selectSnapshotKey(value))]: value }),
        {},
      )
      return undefined
    })
  }

  invalidateAllSnapshots: InvalidateAllSnapshots = () => {
    return later(() => {
      this.snapshots = {}
      return undefined
    })
  }
}

const createInMemSnapshotStore: () => SnapshotStore = () => new Impl()

export const InMemSnapshotStore = {
  of: createInMemSnapshotStore,
}

export const SnapshotStore = {
  noop: noopSnapshotStore,
  inMem: InMemSnapshotStore.of,
}
