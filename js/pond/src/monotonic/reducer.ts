/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { greaterThan } from 'fp-ts/lib/Ord'
import { Event, Events, OffsetMap } from '../eventstore/types'
import { EventKey, LocalSnapshot, StateWithProvenance } from '../types'

export type PendingSnapshot = {
  snap: LocalSnapshot<string>
  tag: string
}

export type Reducer<S> = {
  appendEvents: (
    events: Events,
    emit: boolean,
  ) => {
    snapshots: PendingSnapshot[]
    emit: StateWithProvenance<S>[]
  }

  setState: (state: LocalSnapshot<string>) => void
}

export const stateWithProvenanceReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: LocalSnapshot<S>,
  deserializeState?: (jsonState: unknown) => S,
): Reducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

  const cloneSnap = (snap: LocalSnapshot<S>) => {
    const offsets = { ...snap.psnMap }
    const state = deserialize(JSON.stringify(snap.state))
    return {
      ...snap,
      psnMap: offsets,
      state,
    }
  }

  let head = cloneSnap(initialState)

  let queue = snapshotQueue()

  const snapshotEligible = (_latest: EventKey) => (_snapBase: EventKey) => false

  return {
    appendEvents: (events: Events, emit: boolean) => {
      let { state, psnMap, eventKey } = head

      for (const ev of events) {
        state = onEvent(state, ev)
        psnMap = OffsetMap.update(psnMap, ev)
        eventKey = ev
      }

      head = {
        state,
        psnMap,
        cycle: head.cycle + events.length,
        eventKey,
        horizon: head.horizon, // TODO: Detect new horizons from events
      }

      return {
        snapshots: queue.getSnapsToStore(snapshotEligible(eventKey)),
        // This is for all downstream consumers, so we clone.
        emit: emit ? [cloneSnap(head)] : [],
      }
    },

    setState: snap => {
      if (eventKeyGreater(snap.eventKey, head.eventKey)) {
        // Time travel to future: Reset queue
        queue = snapshotQueue()
      } else {
        // Time travel to the past: All newer cached states are invalid
        queue.invalidateLaterThan(snap.eventKey)
      }

      const oldState = deserialize(snap.state)
      // Clone the input offsets, since they may not be mutable
      head = { ...snap, psnMap: { ...snap.psnMap }, state: oldState }
    },
  }
}

const eventKeyGreater = greaterThan(EventKey.ord)

const snapshotQueue = () => {
  const queue: PendingSnapshot[] = []

  const addPending = (snapshotsAscending: PendingSnapshot[]) => queue.push(...snapshotsAscending)

  const invalidateLaterThan = (cutOff: EventKey) => {
    while (queue.length > 0 && eventKeyGreater(queue[queue.length - 1].snap.eventKey, cutOff)) {
      queue.pop()
    }
  }

  const getSnapsToStore = (storeNow: (snapshotKey: EventKey) => boolean): PendingSnapshot[] => {
    const res = []

    while (queue.length > 0 && storeNow(queue[0].snap.eventKey)) {
      res.push(queue.shift()!)
    }

    return res
  }

  return {
    addPending,
    invalidateLaterThan,
    getSnapsToStore,
  }
}
