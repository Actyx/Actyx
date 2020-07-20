/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { right } from 'fp-ts/lib/Either'
import { contramap, Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import { TagQuery, TypedTagIntersection, Where } from './tagging'
import * as t from 'io-ts'
import { Event, OffsetMap } from './eventstore/types'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isString = (x: any): x is string => typeof x === 'string'
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isNumber = (x: any): x is number => typeof x === 'number'
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isBoolean = (x: any): x is boolean => typeof x === 'boolean'

export type Semantics = string

const internalSemantics = (s: string): Semantics => `internal-${s}` as Semantics
export const Semantics = {
  of(name: string): Semantics {
    if (name.startsWith('jelly-')) {
      throw new Error('Name must not start with jelly-')
    }
    if (name.startsWith('internal-')) {
      throw new Error('Name must not start with internal-')
    }
    return name as Semantics
  },
  jelly: (s: string): Semantics => `jelly-${s}` as Semantics,
  isJelly: (s: Semantics): boolean => s.startsWith('jelly-'),
  internal: internalSemantics,
  isInternal: (s: Semantics): boolean => s.startsWith('internal-'),
  none: internalSemantics('nofish'),
  FromString: new t.Type<Semantics, string>(
    'SemanticsFromString',
    (x): x is Semantics => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as Semantics),
    x => x,
  ),
}

export type FishName = string
export const FishName = {
  of: (s: string): FishName => s as FishName,
  none: 'internal-nofish' as FishName,
  FromString: new t.Type<FishName, string>(
    'FishNameFromString',
    (x): x is FishName => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as FishName),
    x => x,
  ),
}

export type Tags = ReadonlyArray<string>
type TagsOnWire = ReadonlyArray<string> | undefined
export const Tags = new t.Type<Tags, TagsOnWire>(
  'TagsSetFromArray',
  (x): x is Tags => x instanceof Array && x.every(isString),
  // Rust side for now expresses empty tag arrays as omitting the field
  (x, c) => (x === undefined ? right([]) : t.readonlyArray(t.string).validate(x, c)),
  // Sending empty arrays is fine, though
  x => x,
)

export type SourceId = string
const mkSourceId = (text: string): SourceId => text as SourceId
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
export const SourceId = {
  /**
   * Creates a SourceId from a string
   */
  of: mkSourceId,
  /**
   * Creates a random SourceId with the given number of digits
   */
  random: (digits?: number) => mkSourceId(randomBase58(digits || 11)),
  FromString: new t.Type<SourceId, string>(
    'SourceIdFromString',
    (x): x is SourceId => isString(x),
    (x, c) => t.string.validate(x, c).map(s => s as SourceId),
    x => x,
  ),
}

export type Lamport = number
const mkLamport = (value: number): Lamport => value as Lamport
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

export type Psn = number
const mkPsn = (psn: number): Psn => psn as Psn
export const Psn = {
  of: mkPsn,
  zero: mkPsn(0),
  /**
   * A value that is below any valid Psn
   */
  min: mkPsn(-1),
  /**
   * A value that is above any valid Psn
   */
  max: mkPsn(Number.MAX_SAFE_INTEGER),
  FromNumber: new t.Type<Psn, number>(
    'PsnFromNumber',
    (x): x is Psn => isNumber(x),
    (x, c) => t.number.validate(x, c).map(s => s as Psn),
    x => x,
  ),
}

export type Timestamp = number
const mkTimestamp = (time: number): Timestamp => time as Timestamp
const formatTimestamp = (timestamp: Timestamp): string => new Date(timestamp / 1000).toISOString()
const secondsPerDay = 24 * 60 * 60
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

export type Milliseconds = number
const mkMilliseconds = (time: number): Milliseconds => time as Milliseconds
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

/**
 * The source of an event stream: a single localized fish instance
 * characterised by its semantic name, instance name, pond sourceId.
 */
export type Source = Readonly<{
  semantics: Semantics
  name: FishName
  sourceId: SourceId
}>

export type Envelope<E> = {
  readonly source: Source
  readonly lamport: Lamport
  readonly timestamp: Timestamp // Number of microseconds since the unix epoch. Date.now() * 1000
  readonly payload: E
}

const zeroKey: EventKey = {
  lamport: Lamport.zero,
  // Cannot use empty source id, store rejects.
  sourceId: SourceId.of('!'),
  psn: Psn.of(0),
}

const keysEqual = (a: EventKey, b: EventKey): boolean =>
  a.lamport === b.lamport && a.sourceId === b.sourceId

const keysCompare = (a: EventKey, b: EventKey): Ordering => {
  const lamportOrder = ordNumber.compare(a.lamport, b.lamport)
  if (lamportOrder !== 0) {
    return lamportOrder
  }
  return ordString.compare(a.sourceId, b.sourceId)
}

/**
 * Order for event keys
 *
 * Order is [timestamp, sourceId, psn]. Envent keys are considered equal when `timestamp`,
 * `sourceId` and `psn` are equal.
 */
const ordEventKey: Ord<EventKey> = {
  equals: keysEqual,
  compare: keysCompare,
}

const formatEventKey = (key: EventKey): string => `${key.lamport}/${key.sourceId}`

export const EventKey = {
  zero: zeroKey,
  ord: ordEventKey,
  format: formatEventKey,
}
export const EventKeyIO = t.readonly(
  t.type({
    lamport: Lamport.FromNumber,
    psn: Psn.FromNumber,
    sourceId: SourceId.FromString,
  }),
)

export type EventKey = t.TypeOf<typeof EventKeyIO>

export type SnapshotFormat<S, Serialized> = {
  /**
   * This number must be increased whenever:
   *
   * - code changes are made that affect the computed private state
   * - private state type definition is changed
   * - subscription set is changed
   *
   * The version number may remain the same in those rare cases where the new
   * code will seamlessly work with the old snapshots, or if the deserialize
   * function recognizes old snapshot format and converts them to the new one.
   */
  version: number
  /**
   * This function is used to transform the private state into an object that
   * can be serialized using `JSON.stringify()`. In many cases this can be the
   * identity function. Please note that while e.g. immutable Map serializes
   * itself into json automatically, you should still explicitly call `.toJS()`
   * in case the serialization is something else than JSON.stringify(), like
   * e.g. CBOR encoding or storing in indexeddb.
   *
   * In case of function objects within the private state this needs to ensure
   * that the functions can be properly recreated by persisting the values
   * that are captured by the closures.
   */
  serialize: (state: S) => Serialized
  /**
   * A snapshot comes back from the store as the JS object that `serialize`
   * produced, and this function needs to restore it into a proper private
   * state. Please note that while e.g. immutable Map serializes to a proper
   * object by itself, deserialization does NOT yield an immutable Map but
   * just a plain object, so `deserialize` needs to use `Map(obj)`
   * constructor function.
   *
   * In case of a closure, it can be recreated by bringing the needed values
   * into scope and creating an arrow function:
   *
   *     const { paramA, paramB } = (blob as any).some.property
   *     return { myFunc: (x, y) => x * paramA + y * paramB }
   */
  deserialize: (blob: Serialized) => S
}

export const SnapshotFormat = {
  identity: <S>(version: number): SnapshotFormat<S, S> => ({
    version,
    serialize: x => x,
    deserialize: x => x,
  }),
}

/**
 * A state and its corresponding psn map
 */
export type StateWithProvenance<S> = {
  readonly state: S
  /**
   * Minimum psn map that allow to reconstruct the state.
   * Only contains sources that contain events matching the filter.
   */
  readonly psnMap: OffsetMap
}

export type LocalSnapshot<S> = StateWithProvenance<S> & {
  /**
   * eventKey of the last event according to event order that went into the state.
   * This can be used to detect shattering of the state due to time travel.
   */
  eventKey: EventKey

  /**
   * Oldest event key we are interested in. This is defined for a local snapshot
   * that is based on a semantic snapshot. All events before the semantic snapshot
   * that the local snapshot is based on are not relevant and can be discarded.
   *
   * Not discarding these events will lead to unnecessary shattering.
   */
  horizon: EventKey | undefined

  /**
   * Number of events since the beginning of time or the last semantic snapshot (which is
   * kind of the same thing as far as the fish is concerned). This can be used as a measure
   * how useful the snapshot is, and also for count-based snapshot scheduling
   */
  cycle: number
}

export type TaggedIndex = {
  // The index of some array, that we have tagged.
  // It’s mutable because StatePointer<S, E> is meant to be updated when the referenced array changes.
  i: number
  readonly tag: string
  readonly persistAsLocalSnapshot: boolean
}

export const TaggedIndex = {
  ord: contramap((ti: TaggedIndex) => ti.i, ordNumber),
}

export type CachedState<S> = {
  readonly state: StateWithProvenance<S>
  readonly finalIncludedEvent: Event
}

export type StatePointer<S> = TaggedIndex & CachedState<S>

/* 
 * POND V2 APIs
 */
export type Emit<E> = {
  tags: ReadonlyArray<string> | TypedTagIntersection<E>
  payload: E
}

export type Metadata = Readonly<{
  isLocalEvent: boolean
  tags: ReadonlyArray<string>
  timestampMicros: Timestamp
  timestampAsDate: () => Date
  lamport: Lamport
}>

// Combine the existing ("old") state and next event into a new state.
// The returned value may be something completely new, or a mutated version of the input state.
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S
export type IsReset<E> = (event: E, metadata: Metadata) => boolean

// To be refined: generic representation of semantics/name/version for snapshotformat
export type FishId = {
  entityType?: string
  name: string
  version?: number
}

export const FishId = {
  of: (entityType: string, name: string, version: number) => ({
    entityType,
    name,
    version,
  }),
  // Is there an even better way?
  canonical: (v: FishId): string => JSON.stringify([v.entityType, v.name, v.version]),
}

/**
 * A `Fish<S, E>` describes an ongoing aggregration (fold) of events of type `E` into state of type `S`.
 */
export type Fish<S, E> = {
  // Will extend this field with further options in the future:
  // - <E>-Typed subscription
  // - Plain query string
  where: TagQuery | Where<E>

  initialState: S
  onEvent: Reduce<S, E>
  fishId: FishId

  // semantic snapshot
  isReset?: IsReset<E>

  // let’s say we require users to implement .toJSON() on their state for serialisation --
  // then we only need the reverse function. Still a topic of debate: https://github.com/Actyx/Cosmos/issues/2928
  deserializeState?: (jsonState: unknown) => S
}

export const Fish = {
  latestEvent: <E>(where: TagQuery): Fish<E | undefined, E> => ({
    where,

    initialState: undefined,

    onEvent: (_state: E | undefined, event: E) => event,

    fishId: FishId.of('actyx.lib.latestEvent', JSON.stringify(where), 1),

    isReset: () => true,
  }),

  eventsDescending: <E>(where: TagQuery, capacity = 100): Fish<E[], E> => ({
    where,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.unshift(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    fishId: FishId.of('actyx.lib.eventsDescending', JSON.stringify(where), 1),
  }),

  eventsAscending: <E>(where: TagQuery, capacity = 100): Fish<E[], E> => ({
    where,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.push(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    fishId: FishId.of('actyx.lib.eventsAscending', JSON.stringify(where), 1),
  }),
}

export type EmissionRequest<E> = ReadonlyArray<Emit<E>> | Promise<ReadonlyArray<Emit<E>>>

export type StateEffect<S, EWrite> = (state: S) => EmissionRequest<EWrite>

/**
 * Cancel an ongoing aggregation (the provided callback will stop being called).
 */
export type CancelSubscription = () => void

/**
 * Allows you to register actions for when event emission has completed.
 */
export type PendingEmission = {
  subscribe: (whenEmitted: () => void) => void
  toPromise: () => Promise<void>
}
