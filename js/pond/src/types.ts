/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { contramap, Ord, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Ordering } from 'fp-ts/lib/Ordering'
import * as t from 'io-ts'
import { Event, OffsetMap } from './eventstore/types'
import { Tags, Where } from './tagging'

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
// Generic Metadata attached to every event.
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
  eventId: string
}>

// Combine the existing ("old") state and next event into a new state.
// The returned value may be something completely new, or a mutated version of the input state.
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S

// A function indicating events which completely determine the state.
// Any event for which isReset returns true will be applied to the initial state, all earlier events discarded.
export type IsReset<E> = (event: E, metadata: Metadata) => boolean

/**
 * Unique identifier for a fish.
 */
export type FishId = {
  // A general description for the class of thing the Fish represents, e.g. 'robot'
  entityType?: string

  // Concrete name of the represented thing, e.g. 'superAssembler2000'
  name: string

  // Version of the underlying code. Must be increased whenever the Fish’s underlying logic or event selection changes.
  // No version given implies version = 0.
  version?: number
}

/**
 * Unique identifier for a fish.
 */
export const FishId = {
  /**
   * Create a FishId from three components.
   *
   * @param entityType   A general description for the class of thing the Fish represents, e.g. 'robot'
   * @param name         Concrete name of the represented thing, e.g. 'superAssembler2000'
   * @param version      Version of the underlying code. Must be increased whenever the Fish’s underlying logic or event selection changes.
   * @returns            A FishId.
   */
  of: (entityType: string, name: string, version: number) => ({
    entityType,
    name,
    version,
  }),

  // For internal use. Transform a FishId into a string to be used as key in caching.
  canonical: (v: FishId): string => JSON.stringify([v.entityType, v.name, v.version || 0]),
}

/**
 * A `Fish<S, E>` describes an ongoing aggregration (fold) of events of type `E` into state of type `S`.
 * A Fish always sees events in the correct order, even though event delivery on ActyxOS is only eventually consistent:
 * To this effect, arrival of an hitherto unknown event "from the past" will cause a replay of the aggregation
 * from an earlier state, instead of passing that event to the Fish out of order.
 */
export type Fish<S, E> = {
  /**
   * Selection of events to aggregate in this Fish.
   * You may specify plain strings inline: `where: Tags('my', 'tag', 'selection')`
   * Or refer to typed static tags: `where: myFirstTag.and(mySecondTag).or(myThirdTag)`
   */
  where: Where<E>

  // State of this Fish before it has seen any events.
  initialState: S

  /*
   * Function to create the next state from previous state and next event. It works similar to `Array.reduce`.
   * Do note however that this function must be _pure_:
   * - It should not cause any side-effects (except logging)
   * - It should not reference dynamic outside state like random numbers or the current time. The result must depend purely on the input parameters.
   */
  onEvent: Reduce<S, E>

  // Unique identifier for this fish. This is used to enable caching and other performance benefits.
  fishId: FishId

  // Optional: A function indicating events which completely determine the state.
  // Any event for which isReset returns true will be applied to the initial state, all earlier events discarded.
  isReset?: IsReset<E>

  // Custom deserialisation method for your state.
  // The Pond snapshots your state at periodic intervals and persists to disk, to increase performance.
  // Serialisation is done via JSON. To enable custom serialisation, implement `toJSON` on your state.
  // To turn a custom-serialised state back into its proper type, set `deserializeState`.
  deserializeState?: (jsonState: unknown) => S
}

export const Fish = {
  // Observe latest event matching the given selection.
  latestEvent: <E>(where: Where<E>): Fish<E | undefined, E> => ({
    where,

    initialState: undefined,

    onEvent: (_state: E | undefined, event: E) => event,

    fishId: FishId.of('actyx.lib.latestEvent', JSON.stringify(where), 1),

    isReset: () => true,
  }),

  // Observe latest `capacity` events matching given selection, in descending order.
  eventsDescending: <E>(where: Where<E>, capacity = 100): Fish<E[], E> => ({
    where,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.unshift(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    fishId: FishId.of('actyx.lib.eventsDescending', JSON.stringify(where), 1),
  }),

  // Observe latest `capacity` events matching given selection, in ascending order.
  eventsAscending: <E>(where: Where<E>, capacity = 100): Fish<E[], E> => ({
    where,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.push(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    fishId: FishId.of('actyx.lib.eventsAscending', JSON.stringify(where), 1),
  }),
}

// Queue emission of an event whose type is covered by `EWrite`.
export type AddEmission<EWrite> = <E extends EWrite>(tags: Tags<E>, payload: E) => void

// Enqueue event emissions based on currently known local state.
export type StateEffect<S, EWrite> = (
  // Currently known state, including application of all events previously enqueued by state effects on the same Fish.
  state: S,
  // Queue an event for emission. Can be called any number of times.
  enqueue: AddEmission<EWrite>,
) => void | Promise<void>

/**
 * Cancel an ongoing aggregation (the provided callback will stop being called).
 */
export type CancelSubscription = () => void

/**
 * Allows you to register actions for when event emission has completed.
 */
export type PendingEmission = {
  // Add another callback; if emission has already completed, the callback will be executed straight-away.
  subscribe: (whenEmitted: () => void) => void
  // Convert to a Promise which resolves once emission has completed.
  toPromise: () => Promise<void>
}
