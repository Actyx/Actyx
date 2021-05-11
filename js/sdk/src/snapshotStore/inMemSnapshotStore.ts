/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { SnapshotStore } from '../snapshotStore/snapshotStore'
import { EventKey, Lamport, Offset, OffsetMap, StreamId } from '../types'
import {
  InvalidateAllSnapshots,
  InvalidateSnapshots,
  LocalSnapshotFromIndex,
  RetrieveSnapshot,
  StoreSnapshot,
} from './snapshotStore'

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
