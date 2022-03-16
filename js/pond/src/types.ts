/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import {
  isString,
  Lamport,
  Metadata,
  Milliseconds,
  NodeId,
  StateWithProvenance,
  TaggedTypedEvent,
  Tags,
  Timestamp,
  Where,
} from '@actyx/sdk'
import { contramap } from 'fp-ts/lib/Ord'
import { Ord as OrdNumber } from 'fp-ts/lib/number'
import { map as mapE } from 'fp-ts/lib/Either'
import * as t from 'io-ts'
import { Pond } from '.'

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
  none: '_t_' as Semantics,
  FromString: new t.Type<Semantics, string>(
    'SemanticsFromString',
    (x): x is Semantics => isString(x),
    (x, c) => mapE((s) => s as Semantics)(t.string.validate(x, c)),
    (x) => x,
  ),
}

export type FishName = string
export const FishName = {
  of: (s: string): FishName => s as FishName,
  none: '_t_' as FishName,
  FromString: new t.Type<FishName, string>(
    'FishNameFromString',
    (x): x is FishName => isString(x),
    (x, c) => mapE((s) => s as FishName)(t.string.validate(x, c)),
    (x) => x,
  ),
}

/**
 * The source of an event stream: a single localized fish instance
 * characterised by its semantic name, instance name, pond sourceId.
 */
export type Source = {
  semantics: Semantics
  name: FishName
  sourceId: NodeId
}

export type Envelope<E> = {
  readonly source: Source
  readonly lamport: Lamport
  readonly timestamp: Timestamp // Number of microseconds since the unix epoch. Date.now() * 1000
  readonly payload: E
}

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
    serialize: (x) => x,
    deserialize: (x) => x,
  }),
}

export type TaggedIndex = {
  // The index of some array, that we have tagged.
  // It’s mutable because StatePointer<S, E> is meant to be updated when the referenced array changes.
  i: number
  readonly tag: string
  readonly persistAsLocalSnapshot: boolean
}

export const TaggedIndex = {
  ord: contramap((ti: TaggedIndex) => ti.i)(OrdNumber),
}

export type CachedState<S> = {
  readonly state: StateWithProvenance<S>
  readonly finalIncludedEvent: Event
}

export type StatePointer<S> = TaggedIndex & CachedState<S>

/*
 * POND V2 APIs
 */

/**
 * Combine the existing ("old") state and next event into a new state.
 * The returned value may be something completely new, or a mutated version of the input state.
 * @public
 */
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S

/**
 * A function indicating events which completely determine the state.
 * Any event for which isReset returns true will be applied to the initial state, all earlier events discarded.
 * @public
 */
export type IsReset<E> = (event: E, metadata: Metadata) => boolean

/**
 * Unique identifier for a fish.
 * @public
 */
export type FishId = {
  // A general description for the class of thing the Fish represents, e.g. 'robot'
  entityType: string

  // Concrete name of the represented thing, e.g. 'superAssembler2000'
  name: string

  // Version of the underlying code. Must be increased whenever the Fish’s underlying logic or event selection changes.
  version: number
}

/**
 * FishId associated functions.
 * @public
 */
export const FishId = {
  /**
   * Create a FishId from three components.
   *
   * @param entityType - A general description for the class of thing the Fish represents, e.g. 'robot'
   * @param name       - Concrete name of the represented thing, e.g. 'superAssembler2000'
   * @param version    - Version of the underlying code. Must be increased whenever the Fish’s underlying logic or event selection changes.
   * @returns            A FishId.
   */
  of: (entityType: string, name: string, version: number) => {
    if (!entityType || !name) {
      throw new Error('Fish-Id parts must not be left empty')
    }

    return {
      entityType,
      name,
      version,
    }
  },

  // For internal use. Transform a FishId into a string to be used as key in caching.
  canonical: (v: FishId): string => JSON.stringify([v.entityType, v.name, v.version]),
}

/** Indicate in-process (nonpersistent) Caching. @beta */
export type InProcessCaching = {
  type: 'in-process'

  /* Cache key used to find previously stored values */
  key: string
}

/** Indicator for disabled caching of pond.observeAll(). @beta */
export type NoCaching = { readonly type: 'none' }

/** Caching indicator for pond.observeAll(). @beta */
export type Caching = NoCaching | InProcessCaching

export type EnabledCaching = InProcessCaching

/** Caching related functions @beta */
export const Caching = {
  none: { type: 'none' as const },

  isEnabled: (c: Caching | undefined): c is EnabledCaching => c !== undefined && c.type !== 'none',

  inProcess: (key: string): Caching => ({
    type: 'in-process',
    key,
  }),
}

/** Optional parameters to pond.observeAll @beta */
export type ObserveAllOpts = Partial<{
  /**
   * How to cache the known set of Fish.
   * Defaults to no caching, i.e. the set will be rebuilt from events on every invocation.
   */
  caching: Caching

  /** Fish expires from the set of 'all' when its first event reaches a certain age */
  expireAfterSeed: Milliseconds

  /**
   * @deprecated Renamed to `expireAfterSeed`
   */
  expireAfterFirst: Milliseconds

  // Future work: expireAfterLatest(Milliseconds), expireAfterEvent(Where)
}>

/**
 * A `Fish<S, E>` describes an ongoing aggregration (fold) of events of type `E` into state of type `S`.
 * A Fish always sees events in the correct order, even though event delivery on Actyx is only eventually consistent:
 * To this effect, arrival of an hitherto unknown event "from the past" will cause a replay of the aggregation
 * from an earlier state, instead of passing that event to the Fish out of order.
 * @public
 */
export type Fish<S, E> = {
  /**
   * Selection of events to aggregate in this Fish.
   * You may specify plain strings inline: `where: Tags('my', 'tag', 'selection')` (which requires all three tags)
   * Or refer to typed static tags: `where: myFirstTag.and(mySecondTag).or(myThirdTag)`
   * In both cases you would select events which contain all three given tags.
   */
  where: Where<E>

  // State of this Fish before it has seen any events.
  initialState: S

  /**
   * Function to create the next state from previous state and next event. It works similar to `Array.reduce`.
   * Do note however that — while it may modify the passed-in state — this function must be _pure_:
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

/**
 * Fish generic generator methods.
 * @public
 */
export const Fish = {
  // Observe latest event matching the given selection.
  latestEvent: <E>(where: Where<E>): Fish<E | undefined, E> => ({
    where,

    initialState: undefined,

    onEvent: (_state: E | undefined, event: E) => event,

    fishId: FishId.of('actyx.lib.latestEvent', where.toString(), 1),

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

    fishId: FishId.of('actyx.lib.eventsDescending', where.toString(), 1),
  }),

  // Observe latest `capacity` events matching given selection, in ascending order.
  eventsAscending: <E>(where: Where<E>, capacity = 100): Fish<E[], E> => ({
    where,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.push(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    fishId: FishId.of('actyx.lib.eventsAscending', where.toString(), 1),
  }),
}

/**
 * Queue emission of an event whose type is covered by `EWrite`.
 * @public
 */
export type AddEmission<EWrite> = <E extends EWrite>(
  ...args: [Tags<E>, E] | [TaggedTypedEvent<E>]
) => void

/**
 * Enqueue event emissions based on currently known local state.
 * @public
 */
export type StateEffect<S, EWrite> = (
  // Currently known state, including application of all events previously enqueued by state effects on the same Fish.
  state: S,
  // Queue an event for emission. Can be called any number of times.
  enqueue: AddEmission<EWrite>,
  // access to the Pond running this effect, mainly for observing other fishes
  pond: Pond,
) => void | Promise<void>

/** Context for an error thrown by a Fish’s functions. @public */
export type FishErrorContext =
  | { occuredIn: 'onEvent'; state: unknown; event: unknown; metadata: Metadata }
  | { occuredIn: 'isReset'; event: unknown; metadata: Metadata }
  | { occuredIn: 'deserializeState'; jsonState: unknown }

/** Error reporter for when Fish functions throw exceptions. @public */
export type FishErrorReporter = (err: unknown, fishId: FishId, detail: FishErrorContext) => void
