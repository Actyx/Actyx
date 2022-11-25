/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { OffsetMap } from './offsetMap'
import { ActyxEvent, EventKey } from './various'

/// As we are still fleshing out the subscribe_monotonic endpoint, all types in here are alpha.

/**
 * A state and its corresponding psn map.
 * @beta
 */
export type StateWithProvenance<S> = {
  readonly state: S
  /**
   * Minimum psn map that allow to reconstruct the state.
   * Only contains sources that contain events matching the filter.
   */
  readonly offsets: OffsetMap
}

/** A local snapshot of state.
 * @beta */
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

/** A local snapshot where the state has already been serialised.
 * @beta */
export type SerializedStateSnap = LocalSnapshot<string>

/** Possible subscribe_monotonic message types.
 * @alpha */
export enum MsgType {
  state = 'state',
  events = 'events',
  timetravel = 'timetravel',
}

/** Implies consumer should apply the given state.
 * @alpha */
export type StateMsg = {
  type: MsgType.state
  snapshot: SerializedStateSnap
}

/** Implies consumer should apply the given events to its latest local state.
 * @alpha */
export type EventsMsg<E> = {
  type: MsgType.events
  events: ActyxEvent<E>[]
  caughtUp: boolean
}

/** Implies consumer should re-subscribe starting from `trigger` or earlier.
 * @alpha */
export type TimeTravelMsg<E> = {
  type: MsgType.timetravel
  trigger: EventKey
}

/** Possible subscribe_monotonic message types.
 * @alpha */
export type EventsOrTimetravel<E> = StateMsg | EventsMsg<E> | TimeTravelMsg<E>

/**
 * Sent by the client to indicate it wants event delivery to start from this point.
 * Implies that a state was cached in-process by the client and so it does not want to start from a snapshot known to Actyx.
 * @alpha
 */
export type FixedStart = {
  from: OffsetMap
  latestEventKey: EventKey
  horizon?: EventKey
}
