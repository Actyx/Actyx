/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { Lamport, Timestamp } from './types'

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
