/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import { equals } from 'ramda'
import { EventKey, EventKeyIO, NodeId, OffsetMapIO, Where } from '../types'
import { validateOrThrow } from '../util'
import {
  EventStore,
  RequestAllEvents,
  RequestConnectivity,
  RequestOffsets,
  RequestPersistedEvents,
  RequestPersistEvents,
} from './eventStore'
import log from './log'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import {
  AllEventsSortOrder,
  ConnectivityStatus,
  Event,
  Events,
  OffsetsResponse,
  PersistedEventsSortOrder,
  UnstoredEvents,
} from './types'

export const enum RequestTypes {
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
    query: t.string,
    order: PersistedEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type PersistedEventsRequest = t.TypeOf<typeof PersistedEventsRequest>

export const AllEventsRequest = t.readonly(
  t.type({
    offsets: OffsetMapIO,
    minEventKey: EventKeyOrNull,
    upperBound: OffsetMapIO,
    query: t.string,
    order: AllEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type AllEventsRequest = t.TypeOf<typeof AllEventsRequest>

export const PersistEventsRequest = t.readonly(t.type({ data: UnstoredEvents }))
export type PersistEventsRequest = t.TypeOf<typeof PersistEventsRequest>

export const ConnectivityRequest = t.readonly(
  t.type({
    special: t.readonlyArray(NodeId.FromString),
    hbHistDelay: t.number,
    reportEveryMs: t.number, // how frequently the connectivity service should report, recommended around 10_000
    currentPsnHistoryDelay: t.number, // this is u8 size! -- how many report_every_ms spans back we go for the our_psn value? recommended 6 to give 60s
  }),
)
export type ConnectivityRequest = t.TypeOf<typeof ConnectivityRequest>

const EventKeyWithTime = t.intersection([EventKeyIO, t.type({ timestamp: t.number })])
const PublishEventsResponse = t.type({ data: t.readonlyArray(EventKeyWithTime) })

const toAql = (w: Where<unknown>) => 'FROM ' + w.toString()

// FIXME: Downstream consumers expect arrays of Events, but endpoint is no longer sending chunks.
const compat = (x: unknown) => {
  return [x]
}

export class WebsocketEventStore implements EventStore {
  constructor(private readonly multiplexer: MultiplexedWebsocket) {}

  offsets: RequestOffsets = () =>
    this.multiplexer
      .request(RequestTypes.Offsets)
      .map(validateOrThrow(OffsetsResponse))
      .first()
      .toPromise()

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
          query: toAql(whereObj),
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
          offsets: fromPsnsExcluding.psns,
          upperBound: toPsnsIncluding.psns,
          query: toAql(whereObj),
          minEventKey: minEvKey,
          order: sortOrder,
          count: 'max',
        }),
      )
      .map(compat)
      .map(validateOrThrow(Events))
  }

  persistEvents: RequestPersistEvents = events => {
    const publishEvents = events

    return this.multiplexer
      .request(RequestTypes.Publish, PersistEventsRequest.encode({ data: publishEvents }))
      .map(validateOrThrow(PublishEventsResponse))
      .map(({ data: persistedEvents }) => {
        if (publishEvents.length !== persistedEvents.length) {
          log.ws.error(
            'PutEvents: Sent %d events, but only got %d PSNs back.',
            publishEvents.length,
            events.length,
          )
          return []
        }
        return publishEvents.map<Event>((ev, idx) => ({
          ...persistedEvents[idx],
          tags: ev.tags,
          payload: ev.payload,
        }))
      })
      .defaultIfEmpty([])
  }
}
