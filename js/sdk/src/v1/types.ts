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
import { right } from 'fp-ts/lib/Either'
import { Ord } from 'fp-ts/lib/Ord'
import { Ord as OrdString } from 'fp-ts/lib/string'
import { Ord as OrdNumber } from 'fp-ts/lib/number'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { isString } from '../types'

// EnumType Class
export class EnumType<A> extends t.Type<A> {
  public readonly _tag: 'EnumType' = 'EnumType'
  public enumObject!: object
  public constructor(e: object, name?: string) {
    super(
      name || 'enum',
      (u): u is A => Object.values(this.enumObject).some((v) => v === u),
      (u, c) => (this.is(u) ? t.success(u) : t.failure(u, c)),
      t.identity,
    )
    this.enumObject = e
  }
}

// simple helper function
export const createEnumType = <T>(e: object, name?: string) => new EnumType<T>(e, name)

/**
 * Basically adds -infinity and +infinity to a PSN
 */
const PsnOrLimit = t.union([t.number, t.literal('min'), t.literal('max')])
export type PsnOrLimit = t.TypeOf<typeof PsnOrLimit>

/**
 * A psn map with a default value, so it is a tabulated total function from source to PsnOrLimit
 */
export const OffsetMapWithDefault = t.readonly(
  t.type({
    psns: t.readonly(t.record(t.string, t.number)),
    default: PsnOrLimit,
  }),
)
export type OffsetMapWithDefault = t.TypeOf<typeof OffsetMapWithDefault>

const stringRA = t.array(t.string)

type Tags = string[]
type TagsOnWire = string[] | undefined
const Tags = new t.Type<Tags, TagsOnWire>(
  'TagsSetFromArray',
  (x): x is Tags => x instanceof Array && x.every(isString),
  // Rust side for now expresses empty tag arrays as omitting the field
  (x, c) => (x === undefined ? right([]) : stringRA.validate(x, c)),
  // Sending empty arrays is fine, though
  (x) => x,
)

export const EventIO = t.type({
  psn: t.number,
  semantics: t.string,
  sourceId: t.string,
  name: t.string,
  timestamp: t.number,
  lamport: t.number,
  tags: Tags,
  payload: t.unknown,
})

/** A single Actyx Event */
export type Event = t.TypeOf<typeof EventIO>
export const _compareEvents = (a: Event, b: Event): Ordering => {
  const lamportOrder = OrdNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  const sourceOrder = OrdString.compare(a.sourceId, b.sourceId)
  if (sourceOrder !== 0) {
    return sourceOrder
  }
  return OrdNumber.compare(a.psn, b.psn)
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
  compare: _compareEvents,
}

/** Event-related constants */
export const Event = {
  ord: ordEvent,
}

/**
 * A number of events, not necessarily from the same source.
 */
export const Events = t.array(EventIO)

/**
 * A number of events, not necessarily from the same source.
 */
export type Events = t.TypeOf<typeof Events>

/**
 * A number of generated events, that are going to be written to the store.
 */
export const UnstoredEvent = t.readonly(
  t.type({
    /**
     * the sequence nr of the first element in this chunk
     */
    semantics: t.string,
    name: t.string,
    timestamp: t.number,
    tags: Tags,
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
export const PersistedEventsSortOrder: EnumType<PersistedEventsSortOrders> =
  createEnumType<PersistedEventsSortOrders>(PersistedEventsSortOrders, 'PersistedEventsSortOrders')
export type PersistedEventsSortOrder = t.TypeOf<typeof PersistedEventsSortOrder>

/**
 * Sort order for events
 */
export enum AllEventsSortOrders {
  EventKey = 'eventKey',
  Unsorted = 'unsorted',
}
export const AllEventsSortOrder: EnumType<AllEventsSortOrders> =
  createEnumType<AllEventsSortOrders>(AllEventsSortOrders, 'AllEventsSortOrders')
export type AllEventsSortOrder = t.TypeOf<typeof AllEventsSortOrder>

/**
 * Connectivity status type.
 * @public
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
    specialsDisconnected: t.readonlyArray(t.string),
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

/**
 * The IO-TS type parser for ConnectivityStatus.
 * @public
 */
export const ConnectivityStatus = t.union([FullyConnected, PartiallyConnected, NotConnected])

/**
 * Current connectivity of the underlying ActyxOS node.
 * @public
 */
export type ConnectivityStatus = t.TypeOf<typeof ConnectivityStatus>

/* Other things */

/** Hook to run on store connection being closed.
 * @public */
export type StoreConnectionClosedHook = () => void

/** Configuration for the WebSocket store connection.
 * @public */
export type WsStoreConfig = {
  /** url of the destination */
  url: string
  /** protocol of the destination */
  protocol?: string
  /** Hook, when the connection to the store is closed */
  onStoreConnectionClosed?: StoreConnectionClosedHook
  /** retry interval to establish the connection */
  reconnectTimeout?: number

  // todo timeouts?, heartbeats? etc.
}
