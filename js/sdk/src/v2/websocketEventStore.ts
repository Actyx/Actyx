/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import { EventKeyIO, NodeId, OffsetMapIO, Where } from '../types'
import { validateOrThrow } from '../util'
import {
  DoPersistEvents,
  DoQuery,
  DoSubscribe,
  EventStore,
  RequestConnectivity,
  RequestOffsets,
} from './eventStore'
import log from './log'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import {
  ConnectivityStatus,
  Event,
  Events,
  EventsSortOrders,
  OffsetsResponse,
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

const QueryRequest = t.readonly(
  t.type({
    lowerBound: OffsetMapIO,
    upperBound: OffsetMapIO,
    query: t.string,
    order: EventsSortOrders,
  }),
)

const SubscribeRequest = t.readonly(
  t.type({
    offsets: OffsetMapIO,
    query: t.string,
  }),
)

const PersistEventsRequest = t.readonly(t.type({ data: UnstoredEvents }))

const ConnectivityRequest = t.readonly(
  t.type({
    special: t.readonlyArray(NodeId.FromString),
    hbHistDelay: t.number,
    reportEveryMs: t.number, // how frequently the connectivity service should report, recommended around 10_000
    currentPsnHistoryDelay: t.number, // this is u8 size! -- how many report_every_ms spans back we go for the our_psn value? recommended 6 to give 60s
  }),
)

const EventKeyWithTime = t.intersection([EventKeyIO, t.type({ timestamp: t.number })])
const PublishEventsResponse = t.type({ data: t.readonlyArray(EventKeyWithTime) })

const toAql = (w: Where<unknown> | string): string =>
  w instanceof String ? (w as string) : 'FROM ' + w.toString()

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

  query: DoQuery = (lowerBound, upperBound, whereObj, sortOrder) => {
    return this.multiplexer
      .request(
        RequestTypes.Query,
        QueryRequest.encode({
          lowerBound,
          upperBound,
          query: toAql(whereObj),
          order: sortOrder,
        }),
      )
      .map(compat)
      .map(validateOrThrow(Events))
  }

  subscribe: DoSubscribe = (lowerBound, whereObj) => {
    return this.multiplexer
      .request(
        RequestTypes.Subscribe,
        SubscribeRequest.encode({
          offsets: lowerBound,
          query: toAql(whereObj),
        }),
      )
      .map(compat)
      .map(validateOrThrow(Events))
  }

  persistEvents: DoPersistEvents = events => {
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
