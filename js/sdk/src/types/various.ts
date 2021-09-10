/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { contramap, Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import { OffsetMap } from './offsetMap'

/**
 * An Actyx source id.
 * @public
 */
export type NodeId = string
const mkNodeId = (text: string): NodeId => text as NodeId
const randomBase58: (digits: number) => string = (digits: number) => {
  const base58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'.split('')

  let result = ''
  let char

  while (result.length < digits) {
    char = base58[(Math.random() * 57) >> 0]
    result += char
  }
  return result
}

/**
 * `SourceId` associated functions.
 * @public
 */
export const NodeId = {
  /**
   * Creates a NodeId from a string
   */
  of: mkNodeId,
  /**
   * Creates a random SourceId with the given number of digits
   */
  random: (digits?: number) => mkNodeId(randomBase58(digits || 11)),

  streamNo: (nodeId: NodeId, num: number) => nodeId + '-' + num,
}

/**
 * An Actyx stream id.
 * @public
 */
export type StreamId = string
const mkStreamId = (text: string): StreamId => text as StreamId

/**
 * `SourceId` associated functions.
 * @public
 */
export const StreamId = {
  /**
   * Creates a StreamId from a string
   */
  of: mkStreamId,
  /**
   * Creates a random StreamId off a random NodeId.
   */
  random: () => NodeId.streamNo(mkNodeId(randomBase58(11)), Math.floor(Math.random() * 100)),
}

/**
 * An Actyx app id.
 * @public
 */
export type AppId = string
const mkAppId = (text: string): AppId => text as AppId

/**
 * `AppId` associated functions.
 * @public
 */
export const AppId = {
  /**
   * Creates a AppId from a string
   */
  of: mkAppId,
}

/**
 * Lamport timestamp, cf. https://en.wikipedia.org/wiki/Lamport_timestamp
 * @public
 */
export type Lamport = number
const mkLamport = (value: number): Lamport => value as Lamport
/** @public */
export const Lamport = {
  of: mkLamport,
  zero: mkLamport(0),
}

/** Offset within an Actyx event stream. @public */
export type Offset = number
const mkOffset = (n: number): Offset => n as Offset

/** Functions related to Offsets. @public */
export const Offset = {
  of: mkOffset,
  zero: mkOffset(0),
  /**
   * A value that is below any valid Offset
   */
  min: mkOffset(-1),
  /**
   * A value that is above any valid Offset
   */
  max: mkOffset(Number.MAX_SAFE_INTEGER),
}

/** Timestamp (UNIX epoch), MICROseconds resolution. @public */
export type Timestamp = number
const mkTimestamp = (time: number): Timestamp => time as Timestamp
const formatTimestamp = (timestamp: Timestamp): string => new Date(timestamp / 1000).toISOString()
const secondsPerDay = 24 * 60 * 60
/** Helper functions for making sense of and converting Timestamps. @public */
export const Timestamp = {
  of: mkTimestamp,
  zero: mkTimestamp(0),
  maxSafe: mkTimestamp(Number.MAX_SAFE_INTEGER),
  now: (now?: number) => mkTimestamp((now || Date.now()) * 1e3),
  format: formatTimestamp,
  toSeconds: (value: Timestamp) => value / 1e6,
  toMilliseconds: (value: Timestamp): Milliseconds => Milliseconds.of(value / 1e3),
  toDate: (value: Timestamp): Date => new Date(value / 1e3),
  fromDate: (date: Date): Timestamp => mkTimestamp(date.valueOf() * 1e3),
  fromDays: (value: number) => Timestamp.fromSeconds(secondsPerDay * value),
  fromSeconds: (value: number) => mkTimestamp(value * 1e6),
  fromMilliseconds: (value: number) => mkTimestamp(value * 1e3),
  min: (...values: Timestamp[]) => mkTimestamp(Math.min(...values)),
  max: (values: Timestamp[]) => mkTimestamp(Math.max(...values)),
}

/** Some number of milliseconds. @public */
export type Milliseconds = number
const mkMilliseconds = (time: number): Milliseconds => time as Milliseconds
/** Helper functions for making sense of and converting Milliseconds. @public */
export const Milliseconds = {
  of: mkMilliseconds,
  fromDate: (date: Date): Milliseconds => mkMilliseconds(date.valueOf()),
  zero: mkMilliseconds(0),
  now: (now?: number): Milliseconds => mkMilliseconds(now || Date.now()),
  toSeconds: (value: Milliseconds): number => value / 1e3,
  toTimestamp: (value: Milliseconds): Timestamp => Timestamp.of(value * 1e3),
  fromSeconds: (value: number) => mkMilliseconds(value * 1e3),
  fromMinutes: (value: number) => mkMilliseconds(value * 1e3 * 60),
  // Converts millis or micros to millis
  // Note: This is a stopgap until we fixed once and for all this mess.
  fromAny: (value: number): Milliseconds => {
    const digits = Math.floor(Math.abs(value)).toString().length
    return Milliseconds.of(digits <= 13 ? value : value / 1e3)
  },
}

/**
 * Triple that Actyx events are sorted and identified by.
 *
 * @public
 */
export type EventKey = Readonly<{
  // This is not using t.typeof, so that public API has no io-ts dep

  lamport: Lamport
  offset: Offset
  stream: StreamId
}>

const zeroKey: EventKey = {
  lamport: Lamport.zero,
  // Cannot use empty source id, store rejects.
  stream: NodeId.of('!'),
  offset: Offset.of(0),
}

const keysEqual = (a: EventKey, b: EventKey): boolean =>
  a.lamport === b.lamport && a.stream === b.stream

const keysCompare = (a: EventKey, b: EventKey): Ordering => {
  const lamportOrder = ordNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  return ordString.compare(a.stream, b.stream)
}

/**
 * Order for event keys
 *
 * Order is [lamport, streamId, psn]. Event keys are considered equal when `lamport`,
 * `streamId` and `psn` are equal.
 */
const ordEventKey: Ord<EventKey> = {
  equals: keysEqual,
  compare: keysCompare,
}

const formatEventKey = (key: EventKey): string => `${key.lamport}/${key.stream}`

/** Functions related to EventKey. @public */
export const EventKey = {
  zero: zeroKey,
  ord: ordEventKey,
  format: formatEventKey,
}

/** Generic Metadata attached to every event. @public */
export type Metadata = Readonly<{
  // Was this event written by the very node we are running on?
  isLocalEvent: boolean

  // Tags belonging to the event.
  tags: ReadonlyArray<string>

  // Time since Unix Epoch **in Microseconds**!
  timestampMicros: Timestamp

  // Convert the Timestamp to a standard JS Date object.
  timestampAsDate: () => Date

  // Lamport timestamp of the event. Cf. https://en.wikipedia.org/wiki/Lamport_timestamp
  lamport: Lamport

  // A unique identifier for the event.
  // Every event has exactly one eventId which is unique to it, guaranteed to not collide with any other event.
  // Events are *sorted* based on the eventId by Actyx: For a given event, all later events also have a higher eventId according to simple string-comparison.
  eventId: string

  // App id of the event
  appId: AppId

  // Stream this event belongs to
  stream: StreamId

  // Offset of this event inside its stream
  offset: Offset
}>

/** Max length of a lamport timestamp as string. @internal */
export const maxLamportLength = String(Number.MAX_SAFE_INTEGER).length

/** Anthing that has metadata. @internal */
export type HasMetadata = Readonly<{
  timestamp: Timestamp
  lamport: Lamport
  stream: StreamId
  tags: ReadonlyArray<string>
  offset: Offset
  appId: AppId
}>

/** Make a function that makes metadata from an Event as received over the wire. @internal */
export const toMetadata = (sourceId: string) => (ev: HasMetadata): Metadata => ({
  isLocalEvent: ev.stream === sourceId,
  tags: ev.tags,
  timestampMicros: ev.timestamp,
  timestampAsDate: Timestamp.toDate.bind(null, ev.timestamp),
  lamport: ev.lamport,
  appId: ev.appId,
  eventId: String(ev.lamport).padStart(maxLamportLength, '0') + '/' + ev.stream,
  stream: ev.stream,
  offset: ev.offset,
})

/**
 * Cancel an ongoing aggregation (the provided callback will stop being called).
 * @public
 */
export type CancelSubscription = () => void

/**
 * Allows you to register actions for when event emission has completed.
 * @public
 */
export type PendingEmission = {
  // Add another callback; if emission has already completed, the callback will be executed straight-away.
  subscribe: (whenEmitted: (meta: Metadata[]) => void) => void
  // Convert to a Promise which resolves once emission has completed.
  toPromise: () => Promise<Metadata[]>
}

/** An event with tags attached. @public */
export type TaggedEvent = Readonly<{
  tags: string[]
  event: unknown
}>

/** An event with its metadata. @public */
export type ActyxEvent<E = unknown> = {
  meta: Metadata
  payload: E
}

/** Things related to ActyxEvent. @public */
export const ActyxEvent = {
  // TODO: Maybe improve this by just comparing the lamport -> stream combo
  ord: contramap((e: ActyxEvent) => e.meta.eventId, ordString),
}

/**
 * A raw Actyx event to be emitted by the TestEventStore, as if it really arrived from the outside.
 * @public
 */
export type TestEvent = {
  offset: number
  stream: string

  timestamp: Timestamp
  lamport: Lamport
  tags: ReadonlyArray<string>

  payload: unknown
}

/**
 * A chunk of events, with lower and upper bound.
 * A call to `queryKnownRange` with the included bounds is guaranteed to return exactly the contained set of events.
 * A call to `subscribe` with the included `lowerBound`, however, may find new events from sources not included in the bounds.
 *
 * @public
 */
export type EventChunk = {
  /** The event data. Sorting depends on the request which produced this chunk. */
  events: ActyxEvent[]

  /** The lower bound of the event chunk, independent of its sorting in memory. */
  lowerBound: OffsetMap

  /** The upper bound of the event chunk, independent of its sorting in memory. */
  upperBound: OffsetMap
}

/** Options used when creating a new `Actyx` instance. @public */
export type ActyxOpts = {
  /** Host of the Actxy service. This defaults to localhost and should stay localhost in almost all cases. */
  actyxHost?: string

  /** API port of the Actyx service. Defaults to 4454. */
  actyxPort?: number

  /** Hook, when the connection to the store is closed */
  onConnectionLost?: () => void

  /**
   * Whether to try and automatically restore the websocket connection to Actyx, when it is lost
   * Defaults to `false`
   * Note that installing `onConnectionLost(reloadApplication)` is usually more robust than setting `automaticReconnect=true`.
   */
  automaticReconnect?: boolean
}

/** Options used when creating a new TEST `Actyx` instance. @public */
export type ActyxTestOpts = Readonly<{
  /** Local node id to use @public */
  nodeId?: NodeId
}>

/** Manifest describing an Actyx application. Used for authorizing API access. @public */
export type AppManifest = {
  /**
   * Structured application id.
   * For testing and development purposes, you can always pass 'com.example.<somestring>'
   * For production, you will buy a license from Actyx for your specific app id like com.my-company.my-app.
   */
  appId: string

  /** Arbitrary string describing the app. */
  displayName: string

  /** Arbitrary version string */
  version: string

  /** Manifest signature, if it’s not an example app. */
  signature?: string
}

/**
 * Sort order for persisted events.
 * @public
 */
export enum EventsSortOrder {
  /** Strictly ascending, meaning events are strictly ordered by eventId. */
  Ascending = 'asc',

  /** Strictly descending, meaning events are strictly ordered by eventId, reverse. */
  Descending = 'desc',

  /** Ascending per stream, meaning between different streams there is no specific order guaranteed. */
  StreamAscending = 'stream-asc',
}

/** AQL message describing the offsets at the current state of the response stream. @beta */
export type AqlOffsetsMsg = {
  type: 'offsets'
  offsets: OffsetMap
}

/** AQL message conveying a raw event or a record created by mapping an event via SELECT. @beta */
export type AqlEventMessage = {
  /** A simply-mapped event */
  type: 'event'

  /** Payload data generated by the AQL query via SELECT statements. */
  payload: unknown

  /** Metadata of the event that yielded this record. */
  meta: Metadata
}

/** AQL diagnostic output describing an error that occured during query evaluation. @beta */
export type AqlDiagnosticMessage = {
  /** A problem occured */
  type: 'diagnostic'
  severity: string
  message: string
}

/** Future versions of AQL will know additional response message types. @beta */
export type AqlFutureCompat = {
  /** Consult AQL documentation to find out about future available types. */
  type: Exclude<'event' | 'offsets' | 'diagnostic', string>

  payload: unknown

  /** Consult AQL documentation to find out about future metadata contents. */
  meta: Record<string, unknown>
}

/** Response message returned by running AQL query. @beta */
export type AqlResponse = AqlEventMessage | AqlOffsetsMsg | AqlDiagnosticMessage | AqlFutureCompat
