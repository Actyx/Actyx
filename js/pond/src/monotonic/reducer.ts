/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { greaterThan } from 'fp-ts/lib/Ord'
import { Event, Events, OffsetMap } from '../eventstore/types'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { EventKey, LocalSnapshot, StateWithProvenance, Timestamp } from '../types'

export type SerializedStateSnap = LocalSnapshot<string>

export type PendingSnapshot = Readonly<{
  snap: SerializedStateSnap
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

  setState: (state: SerializedStateSnap) => void
}

export const stateWithProvenanceReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: SerializedStateSnap,
  deserializeState?: (jsonState: unknown) => S,
): Reducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

  const deserializeSnapshot = (snap: SerializedStateSnap): LocalSnapshot<S> => {
    const snapState = deserialize(snap.state)
    // Clone the input offsets, since they may not be mutable
    // (TODO: Seems like slightly too much cloning of offsets)
    return { ...snap, psnMap: { ...snap.psnMap }, state: snapState }
  }

  // Head is always the latest state known to us
  let head: LocalSnapshot<S> = deserializeSnapshot(initialState)

  const snapshotHead = (): SerializedStateSnap => ({
    ...head,
    // Clone the mutable parts to avoid interference
    state: JSON.stringify(head.state),
    psnMap: { ...head.psnMap },
  })

  const clonedHead = () => deserializeSnapshot(snapshotHead())

  let queue = snapshotQueue()

  const snapshotScheduler = SnapshotScheduler.create(10)
  const snapshotEligible = (latest: Timestamp) => (snapBase: PendingSnapshot) =>
    snapshotScheduler.isEligibleForStorage(snapBase, { timestamp: latest })

  // Advance the head by applying the given event array between (i ..= iToInclusive)
  const advanceHead = (events: Events, i: number, iToInclusive: number) => {
    if (i > iToInclusive) {
      return
    }

    let { state, psnMap, eventKey, cycle } = head

    while (i <= iToInclusive) {
      const ev = events[i]
      state = onEvent(state, ev)
      psnMap = OffsetMap.update(psnMap, ev)
      eventKey = ev

      i += 1
      cycle += 1
    }

    head = {
      state,
      psnMap,
      cycle,
      eventKey,
      horizon: head.horizon, // TODO: Detect new horizons from events
    }
  }

  const appendEvents = (events: Events, emit: boolean) => {
    // FIXME: Arguments are a bit questionable, but we can’t change the scheduler yet, otherwise the FES-based tests start failing.
    const statesToStore = snapshotScheduler.getSnapshotLevels(head.cycle + 1, events, 0)

    let i = 0
    for (const toStore of statesToStore) {
      advanceHead(events, i, toStore.i)
      i = toStore.i + 1

      queue.addPending({
        snap: snapshotHead(),
        tag: toStore.tag,
        timestamp: events[i].timestamp,
      })
    }

    advanceHead(events, i, events.length - 1)

    const snapshots =
      events.length > 0
        ? queue.getSnapsToStore(snapshotEligible(events[events.length - 1].timestamp))
        : []

    return {
      snapshots,
      // This is for all downstream consumers, so we clone.
      emit: emit ? [clonedHead()] : [],
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

      head = deserializeSnapshot(snap)
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
