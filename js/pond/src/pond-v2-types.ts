/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { TagQuery, TypedTagIntersection, Where } from './tagging'
import { Lamport, Timestamp } from './types'

/* 
 * POND V2 Candidate APIs
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
 * An `Aggregate<S, E>` describes an aggregration of events of type `E` into state of type `S`.
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

export type AnyAggregate = Fish<any, any>

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
