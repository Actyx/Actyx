/* eslint-disable @typescript-eslint/no-non-null-assertion */
import * as assert from 'assert'
import { greaterThan } from 'fp-ts/lib/Ord'
import { Event, Events, OffsetMap } from '../eventstore/types'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { EventKey, LocalSnapshot, StateWithProvenance, Timestamp } from '../types'

export type PendingSnapshot = Readonly<{
  snap: LocalSnapshot<string>
  tag: string
  timestamp: Timestamp
}>

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

  const snapshotScheduler = SnapshotScheduler.create(10)
  const snapshotEligible = (latest: Timestamp) => (snapBase: PendingSnapshot) =>
    snapshotScheduler.isEligibleForStorage(snapBase, { timestamp: latest })

  const appendEvents = (events: Events, emit: boolean) => {
    let { state, psnMap, eventKey } = head

    // FIXME: Arguments are a bit questionable, but we can’t change the scheduler yet, otherwise the FES-based tests start failing.
    const statesToStore = snapshotScheduler.getSnapshotLevels(head.cycle + 1, events, 0)

    let i = 0
    for (const toStore of statesToStore) {
      while (i <= toStore.i) {
        const ev = events[i]
        state = onEvent(state, ev)
        psnMap = OffsetMap.update(psnMap, ev)
        eventKey = ev

        i += 1
      }

      assert(
        // i has been incremented by 1 at the end of the loop, we actually do expect equality
        i - 1 === toStore.i,
        'Expected statesToStore to be in ascending order, with no entries earlier then the latestStored pointer.',
      )

      const psnMapCopy = { ...psnMap }
      const stateWithProvenance = {
        state: JSON.stringify(state),
        psnMap: psnMapCopy,
        cycle: head.cycle + i,
        eventKey,
        horizon: head.horizon, // TODO: Detect new horizons from events
      }

      queue.addPending({
        snap: stateWithProvenance,
        tag: toStore.tag,
        timestamp: events[i].timestamp,
      })
    }

    while (i < events.length) {
      const ev = events[i]
      state = onEvent(state, ev)
      psnMap = OffsetMap.update(psnMap, ev)
      eventKey = ev

      i += 1
    }

    head = {
      state,
      psnMap,
      cycle: head.cycle + events.length,
      eventKey,
      horizon: head.horizon, // TODO: Detect new horizons from events
    }

    const snapshots =
      events.length > 0
        ? queue.getSnapsToStore(snapshotEligible(events[events.length - 1].timestamp))
        : []

    return {
      snapshots,
      // This is for all downstream consumers, so we clone.
      emit: emit ? [cloneSnap(head)] : [],
    }
  }

  return {
    appendEvents,

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

  const addPending = (snap: PendingSnapshot) => queue.push(snap)

  const invalidateLaterThan = (cutOff: EventKey) => {
    while (queue.length > 0 && eventKeyGreater(queue[queue.length - 1].snap.eventKey, cutOff)) {
      queue.pop()
    }
  }

  const getSnapsToStore = (storeNow: (snapshot: PendingSnapshot) => boolean): PendingSnapshot[] => {
    const res = []

    while (queue.length > 0 && storeNow(queue[0])) {
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
