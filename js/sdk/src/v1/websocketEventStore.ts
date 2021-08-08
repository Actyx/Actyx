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
import * as t from 'io-ts'
import { Observable } from '../../node_modules/rxjs'
import {
  DoPersistEvents,
  DoQuery,
  DoSubscribe,
  EventStore,
  RequestOffsets,
} from '../internal_common/eventStore'
import log from '../internal_common/log'
import { EventsSortOrder, Timestamp, Where } from '../types'
import { validateOrThrow } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { SubscriptionSet, SubscriptionSetIO } from './subscription'
import {
  AllEventsSortOrder,
  AllEventsSortOrders,
  Event,
  Events,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  PersistedEventsSortOrders,
  UnstoredEvents,
} from './types'

export const enum RequestTypes {
  SourceId = '/ax/events/getSourceId',
  Present = '/ax/events/requestPresent',
  PersistedEvents = '/ax/events/requestPersistedEvents',
  AllEvents = '/ax/events/requestAllEvents',
  PersistEvents = '/ax/events/persistEvents',
  HighestSeen = '/ax/events/highestSeenOffsets',
  Connectivity = '/ax/events/requestConnectivity',
}
const EventKeyIO = t.readonly(
  t.type({
    lamport: t.number,
    psn: t.number,
    sourceId: t.string,
  }),
)

const EventKeyOrNull = t.union([t.null, EventKeyIO])
const ValueOrLimit = t.union([t.number, t.literal('min'), t.literal('max')])
export type ValueOrLimit = t.TypeOf<typeof ValueOrLimit>
export const PersistedEventsRequest = t.readonly(
  t.type({
    minEventKey: EventKeyOrNull,
    fromPsnsExcluding: OffsetMapWithDefault,
    toPsnsIncluding: OffsetMapWithDefault,
    subscriptionSet: SubscriptionSetIO,
    sortOrder: PersistedEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type PersistedEventsRequest = t.TypeOf<typeof PersistedEventsRequest>

export const AllEventsRequest = t.readonly(
  t.type({
    fromPsnsExcluding: OffsetMapWithDefault,
    minEventKey: EventKeyOrNull,
    toPsnsIncluding: OffsetMapWithDefault,
    subscriptionSet: SubscriptionSetIO,
    sortOrder: AllEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type AllEventsRequest = t.TypeOf<typeof AllEventsRequest>

export const PersistEventsRequest = t.readonly(t.type({ events: UnstoredEvents }))
export type PersistEventsRequest = t.TypeOf<typeof PersistEventsRequest>

export const getSourceId = (multiplexedWebsocket: MultiplexedWebsocket): Promise<string> =>
  multiplexedWebsocket
    .request(RequestTypes.SourceId)
    .map(validateOrThrow(t.string))
    .first()
    .toPromise()

const toSubscriptionSet = (where: Where<unknown>): SubscriptionSet => {
  const wire = where.toV1WireFormat()

  return {
    type: 'tags',
    subscriptions: Array.isArray(wire) ? wire : [wire],
  }
}

const toPersistedSortOrder = (o: EventsSortOrder) => {
  switch (o) {
    case EventsSortOrder.Ascending:
      return PersistedEventsSortOrders.EventKey
    case EventsSortOrder.Descending:
      return PersistedEventsSortOrders.ReverseEventKey
    case EventsSortOrder.StreamAscending:
      return PersistedEventsSortOrders.Unsorted
  }
}

const convertV1toV2 = (e: Event) => {
  const tags = [...e.tags]

  tags.push('semantics:' + e.semantics)
  tags.push('fish_name:' + e.name)

  return {
    appId: 'com.unknown',
    stream: e.sourceId,
    tags,
    payload: e.payload,
    timestamp: e.timestamp,
    lamport: e.lamport,
    offset: e.psn,
  }
}

export class WebsocketEventStore implements EventStore {
  private _present: Observable<OffsetMapWithDefault>
  private _highestSeen: Observable<OffsetMapWithDefault>

  constructor(private readonly multiplexer: MultiplexedWebsocket, readonly sourceId: string) {
    this._present = Observable.defer(() =>
      this.multiplexer.request(RequestTypes.Present).map(validateOrThrow(OffsetMapWithDefault)),
    ).shareReplay(1)

    this._highestSeen = Observable.defer(() =>
      this.multiplexer.request(RequestTypes.HighestSeen).map(validateOrThrow(OffsetMapWithDefault)),
    ).shareReplay(1)
  }

  offsets: RequestOffsets = () =>
    Observable.combineLatest(this._present, this._highestSeen)
      .take(1)
      .toPromise()
      .then(([pres, _hi]) => {
        // FIXME: Calculate toReplicate from highestSeen
        return { present: pres.psns, toReplicate: {} }
      })

  queryUnchecked = () => {
    throw new Error('not implemented for V1')
  }

  query: DoQuery = (lowerBound, upperBound, query, sortOrder) => {
    if (typeof query === 'string') {
      throw new Error('No AQL support in V1')
    }

    return this.multiplexer
      .request(
        RequestTypes.PersistedEvents,
        PersistedEventsRequest.encode({
          fromPsnsExcluding: { psns: lowerBound, default: 'min' },
          toPsnsIncluding: { psns: upperBound, default: 'min' },
          subscriptionSet: toSubscriptionSet(query),
          minEventKey: null,
          sortOrder: toPersistedSortOrder(sortOrder),
          count: 'max',
        }),
      )
      .concatMap(validateOrThrow(Events))
      .map(convertV1toV2)
  }

  subscribe: DoSubscribe = (lowerBound, query) => {
    if (typeof query === 'string') {
      throw new Error('No AQL support in V1')
    }

    return this.multiplexer
      .request(
        RequestTypes.AllEvents,
        AllEventsRequest.encode({
          fromPsnsExcluding: { psns: lowerBound, default: 'min' },
          toPsnsIncluding: { psns: {}, default: 'max' },
          subscriptionSet: toSubscriptionSet(query),
          minEventKey: null,
          sortOrder: AllEventsSortOrders.Unsorted,
          count: 'max',
        }),
      )
      .concatMap(validateOrThrow(Events))
      .map(convertV1toV2)
  }

  persistEvents: DoPersistEvents = eventsV2 => {
    if (eventsV2.length === 0) {
      return Observable.of()
    }

    const events = eventsV2.map(e => ({
      ...e,
      semantics: '_t_',
      name: '_t_',
      timestamp: Timestamp.now(),
    }))

    return this.multiplexer
      .request(RequestTypes.PersistEvents, PersistEventsRequest.encode({ events }))
      .map(validateOrThrow(t.type({ events: Events })))
      .map(({ events: persistedEvents }) => {
        if (events.length !== persistedEvents.length) {
          log.ws.error(
            'PutEvents: Sent %d events, but only got %d PSNs back.',
            events.length,
            persistedEvents.length,
          )
          return []
        }
        return events.map((ev, idx) => ({
          appId: 'com.unknown',
          stream: this.sourceId,
          tags: ev.tags,
          payload: ev.payload,
          timestamp: persistedEvents[idx].timestamp,
          lamport: persistedEvents[idx].lamport,
          offset: persistedEvents[idx].psn,
        }))
      })
      .defaultIfEmpty([])
  }
}
