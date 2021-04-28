import { Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { isNumber, isString } from './functions'

/**
 * An Actyx source id.
 * @public
 */
export type NodeId = string
const mkNodeId = (text: string): NodeId => text as NodeId
export const randomBase58: (digits: number) => string = (digits: number) => {
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
  FromString: new t.Type<NodeId, string>(
    'NodeIdFromString',
    (x): x is NodeId => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as NodeId),
    x => x,
  ),

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
  FromString: new t.Type<NodeId, string>(
    'StreamIdFromString',
    (x): x is StreamId => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as StreamId),
    x => x,
  ),
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
  FromNumber: new t.Type<Lamport, number>(
    'LamportFromNumber',
    (x): x is Lamport => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => mkLamport(s)),
    x => x,
  ),
}

export type Offset = number
const mkOffset = (psn: number): Offset => psn as Offset
export const Offset = {
  of: mkOffset,
  zero: mkOffset(0),
  /**
   * A value that is below any valid Psn
   */
  min: mkOffset(-1),
  /**
   * A value that is above any valid Psn
   */
  max: mkOffset(Number.MAX_SAFE_INTEGER),
  FromNumber: new t.Type<Offset, number>(
    'PsnFromNumber',
    (x): x is Offset => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => s as Offset),
    x => x,
  ),
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
  FromNumber: new t.Type<Timestamp, number>(
    'TimestampFromNumber',
    (x): x is Timestamp => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => s as Timestamp),
    x => x,
  ),
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
  FromNumber: new t.Type<Milliseconds, number>(
    'MilisecondsFromString',
    (x): x is Milliseconds => isNumber(x),
    (x, c) => t.number.validate(x, c).map(mkMilliseconds),
    x => x,
  ),
}

export const EventKeyIO = t.readonly(
  t.type({
    lamport: Lamport.FromNumber,
    offset: Offset.FromNumber,
    stream: NodeId.FromString,
  }),
)

export type EventKey = t.TypeOf<typeof EventKeyIO>

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
  // Events are *sorted* based on the eventId by ActyxOS: For a given event, all later events also have a higher eventId according to simple string-comparison.
  eventId: string
}>

export const maxLamportLength = String(Number.MAX_SAFE_INTEGER).length

export type HasMetadata = Readonly<{
  timestamp: Timestamp
  lamport: Lamport
  stream: StreamId
  tags: ReadonlyArray<string>
}>

export const toMetadata = (sourceId: string) => (ev: HasMetadata): Metadata => ({
  isLocalEvent: ev.stream === sourceId,
  tags: ev.tags,
  timestampMicros: ev.timestamp,
  timestampAsDate: Timestamp.toDate.bind(null, ev.timestamp),
  lamport: ev.lamport,
  eventId: String(ev.lamport).padStart(maxLamportLength, '0') + '/' + ev.stream,
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
  subscribe: (whenEmitted: () => void) => void
  // Convert to a Promise which resolves once emission has completed.
  toPromise: () => Promise<void>
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
