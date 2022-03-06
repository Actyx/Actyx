/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import {
  DoPersistEvents,
  DoQuery,
  DoSubscribe,
  DoSubscribeMonotonic,
  EventStore,
  RequestOffsets,
  TypedMsg,
} from '../internal_common/eventStore'
import log from '../internal_common/log'
import {
  Event,
  EventIO,
  EventsSortOrders,
  OffsetsResponse,
  SubscribeMonotonicResponseIO,
  UnstoredEvents,
} from '../internal_common/types'
import { AppId, EventsSortOrder, Where, OffsetMap } from '../types'
import { EventKeyIO, OffsetMapIO } from '../types/wire'
import { validateOrThrow } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { lastValueFrom } from '../../node_modules/rxjs'
import { map, filter, defaultIfEmpty, first, tap } from '../../node_modules/rxjs/operators'
import { gte } from 'semver'

export const enum RequestTypes {
  Offsets = 'offsets',
  Query = 'query',
  Subscribe = 'subscribe',
  Publish = 'publish',
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
    lowerBound: OffsetMapIO,
    query: t.string,
  }),
)

const SubscribeMonotonicRequest = t.readonly(
  t.type({
    session: t.string,
    query: t.string,
    lowerBound: OffsetMapIO,
  }),
)

const PersistEventsRequest = t.readonly(t.type({ data: UnstoredEvents }))

const EventKeyWithTime = t.intersection([EventKeyIO, t.type({ timestamp: t.number })])
const PublishEventsResponse = t.type({ data: t.readonlyArray(EventKeyWithTime) })

export class WebsocketEventStore implements EventStore {
  constructor(
    private readonly multiplexer: MultiplexedWebsocket,
    private readonly appId: AppId,
    private readonly currentActyxVersion: () => string,
  ) {}

  offsets: RequestOffsets = () =>
    lastValueFrom(
      this.multiplexer
        .request(RequestTypes.Offsets)
        .pipe(map(validateOrThrow(OffsetsResponse)), first()),
    )

  queryUnchecked = (aqlQuery: string, sortOrder: EventsSortOrder, lowerBound?: OffsetMap) =>
    this.multiplexer
      .request(RequestTypes.Query, {
        lowerBound: lowerBound || {},
        query: aqlQuery,
        order: sortOrder,
      })
      .pipe(
        tap({
          next: (item) =>
            log.ws.debug(`got queryUnchecked response of type '${(<TypedMsg>item).type}'`),
          error: (err) => log.ws.info('queryUnchecked response stream failed', err),
          complete: () => log.ws.debug('queryUnchecked reponse completed'),
        }),
        map((x) => x as TypedMsg),
      )

  query: DoQuery = (lowerBound, upperBound, whereObj, sortOrder, horizon) =>
    this.multiplexer
      .request(
        RequestTypes.Query,
        QueryRequest.encode({
          lowerBound,
          upperBound,
          query: `FEATURES(eventKeyRange) FROM (${whereObj}) ${
            gte(this.currentActyxVersion(), '2.5.0') && horizon ? `& from(${horizon})` : ''
          }`,
          order: sortOrder,
        }),
      )
      .pipe(
        tap({
          next: (item) => log.ws.debug(`got query response of type '${(<TypedMsg>item).type}'`),
          error: (err) => log.ws.info('query response stream failed', err),
          complete: () => log.ws.debug('query reponse completed'),
        }),
        filter((x) => (x as TypedMsg).type === 'event'),
        map(validateOrThrow(EventIO)),
      )

  subscribe: DoSubscribe = (lowerBound, whereObj, horizon) =>
    this.multiplexer
      .request(
        RequestTypes.Subscribe,
        SubscribeRequest.encode({
          lowerBound,
          query: `FEATURES(eventKeyRange) FROM (${whereObj}) ${
            gte(this.currentActyxVersion(), '2.5.0') && horizon ? `& from(${horizon})` : ''
          }`,
        }),
      )
      .pipe(
        tap({
          next: (item) => log.ws.debug(`got subscribe response of type '${(<TypedMsg>item).type}'`),
          error: (err) => log.ws.info('subscribe response stream failed', err),
          complete: () => log.ws.debug('subscribe response completed'),
        }),
        filter((x) => (x as TypedMsg).type === 'event'),
        map(validateOrThrow(EventIO)),
      )

  subscribeMonotonic: DoSubscribeMonotonic = (session, lowerBound, whereObj, horizon) =>
    this.multiplexer
      .request(
        RequestTypes.Subscribe,
        SubscribeMonotonicRequest.encode({
          session,
          lowerBound,
          query: `FEATURES(eventKeyRange) FROM (${whereObj}) ${
            gte(this.currentActyxVersion(), '2.5.0') && horizon ? `& from(${horizon})` : ''
          }`,
        }),
      )
      .pipe(
        tap({
          next: (item) => log.ws.debug(`got subscribe response of type '${(<TypedMsg>item).type}'`),
          error: (err) => log.ws.info('subscribe response stream failed', err),
          complete: () => log.ws.debug('subscribe response completed'),
        }),
        filter((x) =>
          ['diagnostic', 'event', 'offsets', 'timeTravel'].includes((x as TypedMsg).type),
        ),
        map(validateOrThrow(SubscribeMonotonicResponseIO)),
      )

  subscribeUnchecked = (aqlQuery: string, lowerBound?: OffsetMap) =>
    this.multiplexer
      .request(RequestTypes.Subscribe, {
        lowerBound: lowerBound === undefined ? {} : lowerBound,
        query: aqlQuery,
      })
      .pipe(
        tap({
          next: (item) =>
            log.ws.debug(`got subscribeUnchecked response of type '${(<TypedMsg>item).type}'`),
          error: (err) => log.ws.info('subscribeUnchecked response stream failed', err),
          complete: () => log.ws.debug('subscribeUnchecked reponse completed'),
        }),
        map((x) => x as TypedMsg),
      )

  persistEvents: DoPersistEvents = (events) => {
    const publishEvents = events

    return this.multiplexer
      .request(RequestTypes.Publish, PersistEventsRequest.encode({ data: publishEvents }))
      .pipe(
        map(validateOrThrow(PublishEventsResponse)),
        map(({ data: persistedEvents }) => {
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
            appId: this.appId,
            tags: ev.tags,
            payload: ev.payload,
          }))
        }),
        defaultIfEmpty([]),
      )
  }
}
