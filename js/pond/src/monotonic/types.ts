import { Option } from 'fp-ts/lib/Option'
import { Events } from '../eventstore/types'
import { EventKey, LocalSnapshot, StateWithProvenance, Timestamp } from '../types'

// A local snapshot where the state has already been serialised
export type SerializedStateSnap = LocalSnapshot<string>

// A local snapshot pending persistence
export type PendingSnapshot = Readonly<{
  snap: SerializedStateSnap
  tag: string
  timestamp: Timestamp
}>

// A Reducer for the Events-Or-State message stream that
// - is unaware of serialization
// - can apply event chunks within bounds
export type SimpleReducer<S> = {
  // Apply the events between fromIdx and toIdxInclusive to the state
  appendEvents: (events: Events, fromIdx: number, toIdxInclusive: number) => LocalSnapshot<S>

  // Directly set the state
  setState: (state: LocalSnapshot<S>) => void
}

// A Reducer for the Events-Or-State message stream
// which will cache intermediate States.
// (This type would extend `Reducer`, if JS had 0-copy slices.)
export type CachingReducer<S> = {
  // Apply the given events to the state and return the new latest state.
  // Internally, this will queue up snapshots and persist them async (persistence can be awaited via awaitPendingPersistence)
  appendEvents: (events: Events) => StateWithProvenance<S>

  // Just directly set a certain state.
  // Will invalidate all later cached states.
  setState: (state: SerializedStateSnap) => void

  // Await persistence for all states (local snapshots)
  awaitPendingPersistence: () => Promise<void>

  // Get the latest state still valid according to the given arguments.
  // This method by itself does not invalidate anything inside the cache, it just filters.
  latestKnownValidState: (
    invalidateStatesAfter: EventKey,
    highestTrigger: EventKey,
  ) => Option<SerializedStateSnap>
}
