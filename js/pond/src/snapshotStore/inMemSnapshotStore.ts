/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { OffsetMap } from '../eventstore'
import { SnapshotStore } from '../snapshotStore/snapshotStore'
import { EventKey, FishName, Lamport, Psn, Semantics, SourceId } from '../types'
import {
  InvalidateAllSnapshots,
  InvalidateSnapshots,
  LocalSnapshotFromIndex,
  RetrieveSnapshot,
  StoreSnapshot,
} from './snapshotStore'

const later: <T>(block: () => T) => Promise<T> = block => Promise.resolve().then(block)

type SnapshotKey = Readonly<{
  semantics: Semantics
  name: FishName
  version: number
  tag: string
}>

type SnapshotRow = SnapshotKey &
  Readonly<{
    lamport: Lamport
    source: SourceId
    psn: Psn
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
  semantics: Semantics,
  name: FishName,
  key: EventKey,
  version: number,
  tag: string,
  blob: string,
  psnMap: OffsetMap,
  horizon: EventKey | undefined,
  cycle: number,
): SnapshotRow => ({
  semantics,
  name,
  lamport: key.lamport,
  source: key.sourceId,
  psn: key.psn,
  version,
  tag,
  blob,
  rootsPsn: psnMap,
  horizon,
  cycle,
})

type SnapshotMap = {
  [key: string]: SnapshotRow
}

class Impl implements SnapshotStore {
  private snapshots: SnapshotMap = {}

  storeSnapshot: StoreSnapshot = (
    sem: Semantics,
    name: FishName,
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

  retrieveSnapshot: RetrieveSnapshot = (s: Semantics, n: FishName, v: number) => {
    const reverseKeyOrder = (l: SnapshotRow, r: SnapshotRow) =>
      EventKey.ord.compare({ ...r, sourceId: r.source }, { ...l, sourceId: l.source })
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
          sourceId: snap.source,
          psn: snap.psn,
        }
        const psnMap: OffsetMap = snap.rootsPsn
        // psnMap is readonly in production.
        Object.preventExtensions(psnMap)
        const horizon = snap.horizon
        const cycle = snap.cycle
        return { eventKey, state: snap.blob, psnMap, horizon, cycle }
      }
    })
  }

  invalidateSnapshots: InvalidateSnapshots = (sem: Semantics, name: FishName, key: EventKey) => {
    return later(() => {
      const snapshotAfterOrOnEventKey = (
        lamport: Lamport,
        source: SourceId,
        psn: Psn,
        eventKey: EventKey,
      ): boolean => {
        const snapshotEventKey: EventKey = { lamport, sourceId: source, psn }
        return EventKey.ord.compare(snapshotEventKey, eventKey) >= 0
      }
      const values = Object.values(this.snapshots)
      const newValues = values.filter(
        sr =>
          !(
            sr.semantics === sem &&
            sr.name === name &&
            snapshotAfterOrOnEventKey(sr.lamport, sr.source, sr.psn, key)
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
