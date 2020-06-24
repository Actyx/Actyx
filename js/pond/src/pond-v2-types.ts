/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { Lamport, Timestamp } from './types'
import { PondCommon } from './pond-common'

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
  timestampMicros: Timestamp
  timestampAsDate: () => Date
  lamport: Lamport
  // TODO: Add more.
}>

// Combine the existing ("old") state and next event into a new state.
// The returned value may be something completely new, or a mutated version of the input state.
export type Reduce<S, E> = (state: S, event: E, metadata: Metadata) => S

// To be refined: generic representation of semantics/name/version for snapshotformat
export type EntityId = {
  entityType?: string
  name: string
  version?: number
}

export const EntityId = {
  of: (entityType: string, name: string, version: number) => ({
    entityType,
    name,
    version,
  }),
  // Is there an even better way?
  canonical: (v: EntityId): string => JSON.stringify([v.entityType, v.name, v.version]),
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
  entityId: EntityId

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

    entityId: EntityId.of('actyx.lib.latestEvent', JSON.stringify(subscriptions), 1),

    isReset: (_event: E) => true,
  }),

  eventsDescending: <E>(subscriptions: TagQuery, capacity = 100): Aggregate<E[], E> => ({
    subscriptions,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.unshift(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    entityId: EntityId.of('actyx.lib.eventsDescending', JSON.stringify(subscriptions), 1),
  }),

  eventsAscending: <E>(subscriptions: TagQuery, capacity = 100): Aggregate<E[], E> => ({
    subscriptions,

    initialState: [],

    onEvent: (state: E[], event: E) => {
      state.push(event)
      return state.length > capacity ? state.slice(0, capacity) : state
    },

    entityId: EntityId.of('actyx.lib.eventsAscending', JSON.stringify(subscriptions), 1),
  }),
}

export type AnyAggregate = Aggregate<any, any>

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

export type PondV2 = {
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
   * @param requiredTags We select those events which are marked with all of the required tags.
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
   * @param requiredTags We select those events which are marked with all of the required tags.
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
    entityId: EntityId,
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

  /* CONDITIONAL EMISSION (COMMANDS) */

  /**
   * Run a single Effect against the current **locally known** State of the `aggregate`.
   * The Effect is able to consider the current State and create Events from it.
   * Every Effect will see the Events of all previous Effects *on this aggregate* applied already!
   *
   * There are no serialisation guarantees whatsoever with regards to other nodes!
   *
   * @typeParam S        State of the Aggregate, input value to the effect.
   * @typeParam EWrite   Payload type(s) to be returned by the effect.
   * @typeParam ReadBack Whether the Aggregate itself must be able to read the emitted events.
   *
   * @param aggregate    Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @returns            A `PendingEmission` object that can be used to register callbacks with the effect’s completion.
   */
  runStateEffect: <S, EWrite, ReadBack = false>(
    aggregate: Aggregate<S, ReadBack extends true ? EWrite : any>,
    effect: StateEffect<S, EWrite>,
  ) => PendingEmission

  /**
   * Create a handle to pass StateEffects to. Functionality is the same as `runStateEffect`, only that `agg` is bound early.
   *
   * @typeParam S        State of the Aggregate, input value to the effect.
   * @typeParam EWrite   Payload type(s) to be returned by the effect.
   * @typeParam ReadBack Whether the Aggregate itself must be able to read the emitted events.
   *
   * @param aggregate    Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @returns            A `PendingEmission` object that can be used to register callbacks with the effect’s completion.
   */
  getOrCreateCommandHandle: <S, EWrite, ReadBack = false>(
    agg: Aggregate<S, ReadBack extends true ? EWrite : any>,
  ) => (effect: StateEffect<S, EWrite>) => PendingEmission

  /**
   * Install a StateEffect that will be applied automatically whenever the `agg`’s State has changed.
   * Every application will see the previous one’s resulting Events applied to the State already, if applicable;
   * but any number of intermediate States may have been skipped between two applications.
   *
   * The effect can be uninstalled by calling the returned `CancelSubscription`.
   *
   * @typeParam S        State of the Aggregate, input value to the effect.
   * @typeParam EWrite   Payload type(s) to be returned by the effect.
   * @typeParam ReadBack Whether the Aggregate must be able to read the emitted events.
   *
   * @param aggregate    Complete aggregation information.
   * @param effect       A function to turn State into an array of Events. The array may be empty, in order to emit 0 Events.
   * @param autoCancel   Condition on which the automatic effect will be cancelled -- State on which `autoCancel` returns `true`
   *                     will be the first State the effect is *not* applied to anymore.
   * @returns            A `CancelSubscription` object that can be used to cancel the automatic effect.
   */
  installAutomaticEffect: <S, EWrite, ReadBack = false>(
    agg: Aggregate<S, ReadBack extends true ? EWrite : any>,
    effect: StateEffect<S, EWrite>,
    autoCancel?: (state: S) => boolean,
  ) => CancelSubscription
} & PondCommon
