/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

/*
 * POND V2 Candidate APIs
 */

type TagIntersection = Readonly<{
  type: 'intersection'
  tags: ReadonlyArray<string>
  onlyLocalEvents?: boolean
}>

type TagUnion = Readonly<{
  type: 'union'
  tags: ReadonlyArray<string | TagIntersection>
}>

const mkUnion = (...tags: (string | TagIntersection)[]): TagUnion => ({
  type: 'union',
  tags,
})

const mkIntersection = (onlyLocalEvents: boolean) => (...tags: string[]): TagIntersection => ({
  type: 'intersection',
  tags,
  onlyLocalEvents,
})

export type TagQuery = TagUnion | TagIntersection

export const TagQuery = {
  // Set terminology
  intersection: mkIntersection(false),
  intersectionLocal: mkIntersection(true),
  union: mkUnion,

  // "What do I match?" terminology
  requireAll: mkIntersection(false),
  requireAllLocal: mkIntersection(true),
  matchAnyOf: mkUnion,

  // JS Array terminology
  requireEvery: mkIntersection(false),
  requireEveryLocal: mkIntersection(true),
  requireSome: mkUnion,

  // For internal use -- should maybe move somewhere else.
  toWireFormat: (sub: TagQuery) => {
    switch (sub.type) {
      case 'intersection':
        return [
          {
            tags: sub.tags,
            local: !!sub.onlyLocalEvents,
          },
        ]

      case 'union':
        return sub.tags.map(
          s =>
            typeof s === 'string'
              ? { tags: [s], local: false }
              : { tags: s.tags, local: !!s.onlyLocalEvents },
        )
    }
  },
}
export type Emit<E> = {
  tags: ReadonlyArray<string>
  payload: E
}

export type Metadata = Readonly<{
  isLocalEvent: boolean
  tags: ReadonlyArray<string>
  // TODO: Add more.
}>

// Combine the existing ("old") state and next event into a new state.
// The returned value may be something completely new, or a mutated version of the input state.
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S

// To be refined: generic representation of semantics/name/version for snapshotformat
export type CacheKey = {
  entityType?: string
  name: string
  version?: number
}

export const CacheKey = {
  namedAggregate: (entityType: string, name: string, version: number) => ({
    entityType,
    name,
    version,
  }),
  // Is there an even better way?
  canonical: (v: CacheKey): string => JSON.stringify([v.entityType, v.name, v.version]),
}

/**
 * An `Aggregate<S, E>` describes an aggregration of events of type `E` into state of type `S`.
 */
export type Aggregate<S, E> = {
  // Will extend this field with further options in the future:
  // - <E>-Typed subscription
  // - Plain query string
  subscriptions: TagQuery

  initialState: S
  onEvent: Reduce<S, E>
  cacheKey: CacheKey

  // semantic snapshot
  isReset?: (event: E) => boolean

  // let’s say we require users to implement .toJSON() on their state for serialisation --
  // then we only need the reverse function. Still a topic of debate: https://github.com/Actyx/Cosmos/issues/2928
  deserializeState?: (jsonState: unknown) => S
}

export const Aggregate = {
  latestEvent: <E>(subscriptions: TagQuery): Aggregate<E | undefined, E> => ({
    subscriptions,

    initialState: undefined,

    onEvent: (_state: E | undefined, event: E) => event,

    cacheKey: CacheKey.namedAggregate('actyx.lib.latestEvent', JSON.stringify(subscriptions), 1),

    isReset: (_event: E) => true,
  }),

  eventsDescending: <E>(subscriptions: TagQuery, capacity = 100): Aggregate<E[], E> => ({
    subscriptions,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.unshift(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    cacheKey: CacheKey.namedAggregate(
      'actyx.lib.eventsDescending',
      JSON.stringify(subscriptions),
      1,
    ),
  }),

  eventsAscending: <E>(subscriptions: TagQuery, capacity = 100): Aggregate<E[], E> => ({
    subscriptions,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.push(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    cacheKey: CacheKey.namedAggregate(
      'actyx.lib.eventsAscending',
      JSON.stringify(subscriptions),
      1,
    ),
  }),
}

export type AnyAggregate = Aggregate<any, any>

export type StateEffect<S, E> = (state: S) => ReadonlyArray<Emit<E>>
export type SimpleStateEffect<S, E> = (state: S) => ReadonlyArray<E>

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

export interface PondV2 {
  /* EMISSION */

  /**
   * Emit a single event directly.
   *
   * @param tags    Tags to attach to the event.
   * @param payload The event payload.
   * @returns       A `PendingEmission` object that can be used to register
   *                callbacks with the emission’s completion.
   */
  emitEvent(tags: string[], payload: unknown): PendingEmission

  /**
   * Emit a number of events at once.
   *
   * @param emit    The events to be emitted, expressed as objects containing `tags` and `payload`.
   * @returns       A `PendingEmission` object that can be used to register
   *                callbacks with the emission’s completion.
   */
  emitEvents(...emit: ReadonlyArray<Emit<any>>): PendingEmission

  /* AGGREGATION */

  /**
   * Aggregate events into state. Aggregation starts from scratch with every call to this function,
   * i.e. no caching is done whatsoever.
   *
   * @param requiredTags We select those events which contain every one of the required tags.
   * @param initialState The initial state of the aggregation.
   * @param onEvent      How to combine old state and an incoming event into new state.
   * @param callback     Function that will be called whenever a new state becomes available.
   * @returns            A function that can be called in order to cancel the aggregation.
   */
  aggregateUncached<S, E>(
    requiredTags: ReadonlyArray<string>,
    initialState: S,
    onEvent: (state: S, event: E) => S,
    callback: (newState: S) => void,
  ): CancelSubscription

  /**
   * Aggregate events into state. Caching is offered based on the passed `CacheKey`.
   *
   * @param requiredTags We select those events which contain every one of the required tags.
   * @param initialState The initial state of the aggregation.
   * @param onEvent      How to combine old state and an incoming event into new state.
   * @param cacheKey     Object describing the aggregation’s identity. If an aggregation with the same
   *                     identity is already running, that one will be returned rather than everything
   *                     started a second time.
   * @param callback     Function that will be called whenever a new state becomes available.
   * @returns            A function that can be called in order to cancel the aggregation.
   */
  aggregatePlain<S, E>(
    requiredTags: ReadonlyArray<string>,
    initialState: S,
    onEvent: (state: S, event: E) => S,
    cacheKey: CacheKey,
    callback: (newState: S) => void,
  ): CancelSubscription

  /**
   * Aggregate events into state. Caching is done based on the `cacheKey` inside the `aggregate`.
   *
   * @param aggregate    Complete aggregation information.
   * @param callback     Function that will be called whenever a new state becomes available.
   * @returns            A function that can be called in order to cancel the aggregation.
   */
  aggregate<S, E>(aggregate: Aggregate<S, E>, callback: (newState: S) => void): CancelSubscription
}
