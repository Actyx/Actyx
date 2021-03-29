/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import * as t from 'io-ts'
import { equals } from 'ramda'
import { Observable } from 'rxjs'
import log from '../loggers'
import { Where } from '../tagging'
import { EventKey, EventKeyIO, SourceId } from '../types'
import {
  EventStore,
  RequestAllEvents,
  RequestConnectivity,
  RequestHighestSeen,
  RequestPersistedEvents,
  RequestPersistEvents,
  RequestPresent,
} from './eventStore'
import { MultiplexedWebsocket, validateOrThrow } from './multiplexedWebsocket'
import { OffsetMap, OffsetMapIO } from './offsetMap'
import {
  AllEventsSortOrder,
  ConnectivityStatus,
  Event,
  Events,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  UnstoredEvents,
} from './types'

export const enum RequestTypes {
  NodeId = 'node_id',
  Offsets = 'offsets',
  Query = 'query',
  Subscribe = 'subscribe',
  Publish = 'publish',
  // FIXME: These endpoints are not yet available in V2.
  HighestSeen = '/ax/events/highestSeenOffsets',
  Connectivity = '/ax/events/requestConnectivity',
}

const EventKeyOrNull = t.union([t.null, EventKeyIO])
const ValueOrLimit = t.union([t.number, t.literal('min'), t.literal('max')])
export type ValueOrLimit = t.TypeOf<typeof ValueOrLimit>
export const PersistedEventsRequest = t.readonly(
  t.type({
    minEventKey: EventKeyOrNull,
    lowerBound: OffsetMapIO,
    upperBound: OffsetMapIO,
    where: t.string,
    order: PersistedEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type PersistedEventsRequest = t.TypeOf<typeof PersistedEventsRequest>

export const AllEventsRequest = t.readonly(
  t.type({
    lowerBound: OffsetMapIO,
    minEventKey: EventKeyOrNull,
    upperBound: OffsetMapIO,
    where: t.string,
    order: AllEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type AllEventsRequest = t.TypeOf<typeof AllEventsRequest>

export const PersistEventsRequest = t.readonly(t.type({ data: UnstoredEvents }))
export type PersistEventsRequest = t.TypeOf<typeof PersistEventsRequest>

const GetSourceIdResponse = t.type({ nodeId: SourceId.FromString })

export const getSourceId = (multiplexedWebsocket: MultiplexedWebsocket): Promise<SourceId> =>
  multiplexedWebsocket
    .request(RequestTypes.NodeId)
    .map(validateOrThrow(GetSourceIdResponse))
    .map(response => response.nodeId)
    .first()
    .toPromise()

export const ConnectivityRequest = t.readonly(
  t.type({
    special: t.readonlyArray(SourceId.FromString),
    hbHistDelay: t.number,
    reportEveryMs: t.number, // how frequently the connectivity service should report, recommended around 10_000
    currentPsnHistoryDelay: t.number, // this is u8 size! -- how many report_every_ms spans back we go for the our_psn value? recommended 6 to give 60s
  }),
)
export type ConnectivityRequest = t.TypeOf<typeof ConnectivityRequest>

const toAql = (w: Where<unknown>) => w.toString()

// FIXME: Downstream consumers expect arrays of Events, but endpoint is no longer sending chunks.
const compat = (x: unknown) => {
  return [x]
}

export class WebsocketEventStore implements EventStore {
  private _present: Observable<OffsetMap>
  private _highestSeen: Observable<OffsetMapWithDefault>

  constructor(private readonly multiplexer: MultiplexedWebsocket, readonly sourceId: SourceId) {
    this._present = Observable.defer(() =>
      this.multiplexer.request(RequestTypes.Offsets).map(validateOrThrow(OffsetMapIO)),
    ).shareReplay(1)

    this._highestSeen = Observable.defer(() =>
      this.multiplexer.request(RequestTypes.HighestSeen).map(validateOrThrow(OffsetMapWithDefault)),
    ).shareReplay(1)
  }

  // FIXME: Change downstream type to only have "psns"
  present: RequestPresent = () => this._present.map(psns => ({ psns, default: 'max' }))

  highestSeen: RequestHighestSeen = () => this._highestSeen

  connectivityStatus: RequestConnectivity = (
    specialSources,
    hbHistDelayMicros,
    reportEvery,
    currentPsnHistoryDelay,
  ) => {
    const params = ConnectivityRequest.encode({
      special: specialSources,
      hbHistDelay: hbHistDelayMicros,
      reportEveryMs: reportEvery,
      currentPsnHistoryDelay,
    })
    return this.multiplexer
      .request(RequestTypes.Connectivity, params)
      .map(validateOrThrow(ConnectivityStatus))
  }

  persistedEvents: RequestPersistedEvents = (
    fromPsnsExcluding,
    toPsnsIncluding,
    whereObj,
    sortOrder,
    minEventKey,
  ) => {
    const minEvKey =
      minEventKey === undefined || equals(minEventKey, EventKey.zero) ? null : minEventKey
    return this.multiplexer
      .request(
        RequestTypes.Query,
        PersistedEventsRequest.encode({
          lowerBound: fromPsnsExcluding.psns,
          upperBound: toPsnsIncluding.psns,
          where: toAql(whereObj),
          minEventKey: minEvKey,
          order: sortOrder,
          count: 'max',
        }),
      )
      .map(compat)
      .map(validateOrThrow(Events))
  }

  allEvents: RequestAllEvents = (
    fromPsnsExcluding,
    toPsnsIncluding,
    whereObj,
    sortOrder,
    minEventKey,
  ) => {
    const minEvKey =
      minEventKey === undefined || equals(minEventKey, EventKey.zero) ? null : minEventKey
    return this.multiplexer
      .request(
        RequestTypes.Subscribe,
        AllEventsRequest.encode({
          lowerBound: fromPsnsExcluding.psns,
          upperBound: toPsnsIncluding.psns,
          where: toAql(whereObj),
          minEventKey: minEvKey,
          order: sortOrder,
          count: 'max',
        }),
      )
      .map(compat)
      .map(validateOrThrow(Events))
  }

  persistEvents: RequestPersistEvents = events => {
    // extract jelly events, as they must not be sent
    // over the wire to the store
    const publishEvents = events

    return this.multiplexer
      .request(RequestTypes.Publish, PersistEventsRequest.encode({ data: publishEvents }))
      .map(validateOrThrow(t.type({ events: Events })))
      .map(({ events: persistedEvents }) => {
        if (publishEvents.length !== persistedEvents.length) {
          log.ws.error(
            'PutEvents: Sent %d events, but only got %d PSNs back.',
            publishEvents.length,
            events.length,
          )
          return []
        }
        return publishEvents.map<Event>((ev, idx) => ({
          stream: this.sourceId,
          name: ev.name,
          tags: ev.tags,
          payload: ev.payload,
          semantics: ev.semantics,
          timestamp: persistedEvents[idx].timestamp,
          lamport: persistedEvents[idx].lamport,
          offset: persistedEvents[idx].offset,
        }))
      })
      .defaultIfEmpty([])
  }
}
