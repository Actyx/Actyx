/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { failure as failureReporter } from 'io-ts/lib/PathReporter'
import { createEnumType, EnvelopeFromStore } from '../store/util'
import { FishName, Lamport, Psn, Semantics, SourceId, Timestamp } from '../types'
import { OffsetMapIO } from './offsetMap'

export { OffsetMap, OffsetMapBuilder } from './offsetMap'

/**
 * Basically adds -infinity and +infinity to a PSN
 */
const PsnOrLimit = t.union([Psn.FromNumber, t.literal('min'), t.literal('max')])
export type PsnOrLimit = t.TypeOf<typeof PsnOrLimit>

/**
 * A psn map with a default value, so it is a tabulated total function from source to PsnOrLimit
 */
export const OffsetMapWithDefault = t.readonly(
  t.type({
    psns: t.readonly(OffsetMapIO),
    default: PsnOrLimit,
  }),
)
export type OffsetMapWithDefault = t.TypeOf<typeof OffsetMapWithDefault>

export const EventIO = t.type({
  psn: Psn.FromNumber,
  semantics: Semantics.FromString,
  sourceId: SourceId.FromString,
  name: FishName.FromString,
  timestamp: Timestamp.FromNumber,
  lamport: Lamport.FromNumber,
  payload: t.unknown,
})
export type Event = t.TypeOf<typeof EventIO>
const compareEvents = (a: Event, b: Event): Ordering => {
  const lamportOrder = ordNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  const sourceOrder = ordString.compare(a.sourceId, b.sourceId)
  if (sourceOrder !== 0) {
    return sourceOrder
  }
  return ordNumber.compare(a.psn, b.psn)
}

const eventsEqual = (a: Event, b: Event): boolean =>
  // compare numerical fields first since it should be faster
  a.lamport === b.lamport && a.psn === b.psn && a.sourceId === b.sourceId
/**
 * Order for events
 *
 * Order is [lamport, sourceId]
 * Events are considered equal when lamport, sourceId, psn are equal without considering
 * the content of the payload. Having two events that have these fields equal yet a different
 * payload would be a grave bug in our system.
 */
const ordEvent: Ord<Event> = {
  equals: eventsEqual,
  compare: compareEvents,
}
export const Event = {
  ord: ordEvent,
  toEnvelopeFromStore: <E>(ev: Event, decoder?: t.Decoder<unknown, E>): EnvelopeFromStore<E> => ({
    source: { semantics: ev.semantics, name: ev.name, sourceId: ev.sourceId },
    timestamp: ev.timestamp,
    lamport: ev.lamport,
    psn: ev.psn,
    // Just a cast in `NODE_ENV==='production'`
    payload: decoder ? unsafeDecode(ev.payload, decoder) : (ev.payload as E),
  }),
  fromEnvelopeFromStore: <E>(ev: EnvelopeFromStore<E>): Event => ({
    lamport: ev.lamport,
    sourceId: ev.source.sourceId,
    semantics: ev.source.semantics,
    name: ev.source.name,
    timestamp: ev.timestamp,
    psn: ev.psn,
    payload: ev.payload,
  }),
}

/**
 * A number of events, not necessarily from the same source
 */
export const Events = t.readonlyArray(EventIO)
export type Events = t.TypeOf<typeof Events>

/**
 * A number of generated events, that are going to be written to the store.
 */
export const UnstoredEvent = t.readonly(
  t.type({
    /**
     * the sequence nr of the first element in this chunk
     */
    semantics: Semantics.FromString,
    name: FishName.FromString,
    timestamp: Timestamp.FromNumber,
    payload: t.unknown,
  }),
)

export type UnstoredEvent = t.TypeOf<typeof UnstoredEvent>
export const UnstoredEvents = t.readonlyArray(UnstoredEvent)
export type UnstoredEvents = t.TypeOf<typeof UnstoredEvents>

/**
 * Sort order for perstisted events
 */
export enum PersistedEventsSortOrders {
  EventKey = 'eventKey',
  ReverseEventKey = 'reverseEventKey',
  Unsorted = 'unsorted',
}
export const PersistedEventsSortOrder = createEnumType<PersistedEventsSortOrders>(
  PersistedEventsSortOrders,
  'PersistedEventsSortOrders',
)
export type PersistedEventsSortOrder = t.TypeOf<typeof PersistedEventsSortOrder>

/**
 * Sort order for events
 */
export enum AllEventsSortOrders {
  EventKey = 'eventKey',
  Unsorted = 'unsorted',
}
export const AllEventsSortOrder = createEnumType<AllEventsSortOrders>(
  AllEventsSortOrders,
  'AllEventsSortOrders',
)
export type AllEventsSortOrder = t.TypeOf<typeof AllEventsSortOrder>

/**
 * Connectivity status
 */
export enum ConnectivityStatusType {
  FullyConnected = 'FullyConnected',
  PartiallyConnected = 'PartiallyConnected',
  NotConnected = 'NotConnected',
}

const FullyConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.FullyConnected),
    inCurrentStatusForMs: t.number,
  }),
)

const PartiallyConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.PartiallyConnected),
    inCurrentStatusForMs: t.number,
    specialsDisconnected: t.readonlyArray(SourceId.FromString),
    swarmConnectivityLevel: t.number, // Percent*100, e.g. 50% would be 50, not 0.5
    eventsToRead: t.number,
    eventsToSend: t.number,
  }),
)

const NotConnected = t.readonly(
  t.type({
    status: t.literal(ConnectivityStatusType.NotConnected),
    inCurrentStatusForMs: t.number,
    eventsToRead: t.number,
    eventsToSend: t.number,
  }),
)

export const ConnectivityStatus = t.union([FullyConnected, PartiallyConnected, NotConnected])
export type ConnectivityStatus = t.TypeOf<typeof ConnectivityStatus>

/* Other things */
function unsafeDecode<T>(value: unknown, decoder: t.Decoder<unknown, T>): T {
  if (process.env.NODE_ENV !== 'production') {
    return decoder.decode(value).fold(errors => {
      throw new Error(failureReporter(errors).join('\n'))
    }, x => x)
  }
  return value as T
}

export type StoreConnectionClosedHook = () => void

export type WsStoreConfig = Readonly<{
  /** url of the destination */
  url: string
  /** protocol of the destination */
  protocol?: string
  /** Hook, when the connection to the store is closed */
  onStoreConnectionClosed?: StoreConnectionClosedHook
  /** retry interval to establish the connection */
  reconnectTimeout?: number
  // todo timeouts?, heartbeats? etc.
}>
