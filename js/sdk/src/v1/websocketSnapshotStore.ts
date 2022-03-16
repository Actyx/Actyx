/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import {
  InvalidateAllSnapshots,
  InvalidateSnapshots,
  Multiplexer,
  RetrieveSnapshot,
  SnapshotStore,
  StoreSnapshot,
} from '../snapshotStore'
import { EventKeyIO, OffsetMapIO } from '../types/wire'
import { validateOrThrow } from '../util'
import { lastValueFrom } from '../../node_modules/rxjs'
import { map } from '../../node_modules/rxjs/operators'

// TODO: generic io-ts mapping from undefined <-> null

const enum RequestTypes {
  StoreSnapshot = '/ax/snapshots/storeSnapshot',
  RetrieveSnapshot = '/ax/snapshots/retrieveSnapshot',
  InvalidateSnapshots = '/ax/snapshots/invalidateSnapshots',
}

const nullable = <Base extends t.Mixed>(base: Base) => t.union([base, t.null])

export const StoreSnapshotRequest = t.readonly(
  t.type({
    semantics: t.string,
    name: t.string,
    key: EventKeyIO,
    offsets: OffsetMapIO,
    horizon: nullable(EventKeyIO),
    cycle: t.number,
    version: t.number,
    tag: t.string,
    blob: t.string,
  }),
)
export type StoreSnapshotRequest = t.TypeOf<typeof StoreSnapshotRequest>

export const RetrieveSnapshotRequest = t.readonly(
  t.type({
    semantics: t.string,
    name: t.string,
    version: t.number,
  }),
)
export type RetrieveSnapshotRequest = t.TypeOf<typeof RetrieveSnapshotRequest>

export const InvalidateSnapshotsRequest = t.readonly(
  t.type({
    semantics: t.string,
    name: t.string,
    key: EventKeyIO,
  }),
)
export type InvalidateSnapshotsRequest = t.TypeOf<typeof InvalidateSnapshotsRequest>

export const RetrieveSnapshotResponse = t.readonly(
  t.type({
    state: t.string,
    offsets: OffsetMapIO,
    eventKey: EventKeyIO,
    horizon: nullable(EventKeyIO),
    cycle: t.number,
  }),
)
export type RetrieveSnapshotResponse = t.TypeOf<typeof RetrieveSnapshotResponse>

export const RetrieveSnapshotResponseOrNull = t.union([t.null, RetrieveSnapshotResponse])
export type RetrieveSnapshotResponseOrNull = t.TypeOf<typeof RetrieveSnapshotResponseOrNull>

export class WebsocketSnapshotStore implements SnapshotStore {
  storeSnapshot: StoreSnapshot = (
    semantics,
    name,
    key,
    offsets,
    horizon,
    cycle,
    version,
    tag,
    blob,
  ) =>
    lastValueFrom(
      this.multiplexer
        .request(
          RequestTypes.StoreSnapshot,
          StoreSnapshotRequest.encode({
            semantics,
            name,
            key,
            offsets,
            horizon: horizon || null,
            cycle,
            version,
            tag,
            blob,
          }),
        )
        .pipe(map(validateOrThrow(t.boolean))),
    )
  retrieveSnapshot: RetrieveSnapshot = (semantics, name, version) =>
    lastValueFrom(
      this.multiplexer
        .request(
          RequestTypes.RetrieveSnapshot,
          RetrieveSnapshotRequest.encode({
            semantics,
            name,
            version,
          }),
        )
        .pipe(
          map(validateOrThrow(RetrieveSnapshotResponseOrNull)),
          map((x) =>
            x === null
              ? undefined
              : {
                  state: x.state,
                  eventKey: x.eventKey,
                  offsets: x.offsets,
                  horizon: x.horizon || undefined,
                  cycle: x.cycle,
                },
          ),
        ),
    )
  invalidateSnapshots: InvalidateSnapshots = (semantics, name, key) =>
    lastValueFrom(
      this.multiplexer
        .request(
          RequestTypes.InvalidateSnapshots,
          InvalidateSnapshotsRequest.encode({
            semantics,
            name,
            key,
          }),
        )
        .pipe(
          map(validateOrThrow(t.null)),
          map((_) => undefined),
        ),
    )

  invalidateAllSnapshots: InvalidateAllSnapshots = () => Promise.resolve(undefined)
  constructor(private readonly multiplexer: Multiplexer) {}
}
