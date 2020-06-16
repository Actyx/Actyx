/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as assert from 'assert'
import { last } from 'fp-ts/lib/Array'
import { fromNullable, none, Option } from 'fp-ts/lib/Option'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { EnvelopeFromStore } from './store/util'
import { StatePointer, TaggedIndex } from './types'

export const getRecentPointers = (recent: number, spacing: number) => (
  initialCycle: number,
  eventBufferLength: number,
  limit: number,
): TaggedIndex[] => {
  const ptrs = []
  const maxDistanceFromTip = Math.min(eventBufferLength - Math.max(limit, 0), recent)

  const virtualLength = eventBufferLength + initialCycle

  // We want to perpetually layout the same couple of states over the recent window.
  // E.g. recent=32 distance=8 means recycling the same 4 pointers.
  // But we want to avoid assigning *always the same pointer* if we only ever get a small number of new events.
  const tagMod = Math.floor(recent / spacing)

  // Find the next proper multiple of spacing that is more than 1 step away from the tip.
  let distance = virtualLength % spacing
  if (distance < 2) {
    distance += spacing
  }
  let multiple = (virtualLength - distance) / spacing
  for (; distance < maxDistanceFromTip; distance += spacing) {
    const i = eventBufferLength - distance
    const tag = 'mod' + (multiple % tagMod)
    ptrs.push({
      tag,
      i,
      persistAsLocalSnapshot: false,
    })
    multiple -= 1
  }

  return ptrs
}

/* Storage of states that correspond to certain indices in an event array.
 *
 * I.e. for index 10 it would store the state corresponding to an aggregation having run from 0 to
 * 10 inclusive.
 *
 * The wrapped snapshotScheduler is used to find points of special interest where we would like to
 * persist the state to disk/ipfs to speed up subsequent application start-ups ("local snapshots").
 * Since we want to avoid going through the array too often, the local snapshot process is 2-step:
 * First, the scheduler marks potential snapshots. The states at those points are cached. 
 * Then later, the scheduler may indicate that those snapshots are now old enough to actually be
 * applied as local snapshots, meaning the event array will be truncated and the snapshot persisted.
 */
export class StatePointers<S, E> {
  // Our latest known state -- this pointer will always live also in one of the `stores`,
  // it is NOT owned by this var. I.e. it’s effectively a pointer to the latest state pointer.
  private latest: Option<StatePointer<S, E>> = none

  // States cached in memory, never going to be persisted to anywhere.
  private readonly ephemeral: Record<string, StatePointer<S, E>> = {}

  // Local snapshots:
  // To apply at the next opportunity
  private readonly pendingApplication: Record<string, StatePointer<S, E>> = {}
  // To apply after the scheduler greenlights (meaning they are old enough)
  private readonly pendingEligibility: Record<string, StatePointer<S, E>> = {}

  // We keep every StatePointer in EXACTLY ONE of these stores.
  // There may be duplicate indices, but the object must be different, since we modify `i` in place!
  private readonly stores: Readonly<
    [
      Record<string, StatePointer<S, E>>,
      Record<string, StatePointer<S, E>>,
      Record<string, StatePointer<S, E>>
    ]
  > = [this.ephemeral, this.pendingApplication, this.pendingEligibility]

  constructor(
    private readonly snapshotScheduler: SnapshotScheduler,
    private readonly recentWindow: number = 32,
    private readonly recentStateSpacing: number = 8,
    private readonly expensive = false,
  ) {}

  private readonly assignRecent = getRecentPointers(this.recentWindow, this.recentStateSpacing)

  /**
   * Invalidate all stored states above a certain index.
   *
   * @param i The highest unchanged index; every cached state with larger index is going to be
   * discarded.
   */
  public invalidateDownTo(i: number): void {
    if (this.latest.exists(l => i >= l.i)) {
      // Latest is earlier than where we are going down to,
      // so there are no pointers at all to invalidate.
      return
    } else {
      this.latest = none
    }

    let high = -1
    let newLatest

    for (const store of this.stores) {
      const tags = Object.keys(store)

      for (const tag of tags) {
        const ptr = store[tag]

        if (ptr.i > i) {
          delete store[tag]
        } else if (ptr.i > high) {
          high = ptr.i
          newLatest = ptr
        }
      }
    }

    this.latest = fromNullable(newLatest)
  }

  /**
   * Shift all state’s indices back by some amount. To be used when the corresponding event buffer
   * is truncated at the front.
   *
   * @param offset Number to shift by (= number of truncated events). Every state with an index
   * larger than this is going to be straight-up discarded.
   */
  public shiftBack(offset: number): void {
    if (this.latest.exists(l => offset > l.i)) {
      this.latest = none
    }

    for (const store of this.stores) {
      const tags = Object.keys(store)

      for (const tag of tags) {
        const ptr = store[tag]
        ptr.i -= offset
        // It has become irrelevant.
        if (ptr.i < 0) {
          delete store[tag]
        }
      }
    }
  }

  /**
   * @returns The latest cached state, i.e. the one corresponding to the highest index.
   */
  public latestStored(): Option<StatePointer<S, E>> {
    return this.latest
  }

  /**
   * Gives a list of indices it would like to have the states cached for, according to a variety of
   * strategies/considerations.
   *
   * @param cycleStart For the scheduler: event number of index -1
   *
   * @param events The complete current event array. It will not be modified, and be iterated only
   * down to the latest point we already have a state for.
   *
   * @returns List of indices (with tag) in ascending order that should be assigned their respective
   * state. No index will be smaller than that of `latestStored`!
   */
  public getStatesToCache(
    cycleStart: number,
    events: ReadonlyArray<EnvelopeFromStore<E>>,
  ): ReadonlyArray<TaggedIndex> {
    const observedSources = new Set()
    const ptrs: TaggedIndex[] = []

    const limit = this.latest.fold(-1, ti => ti.i)

    // Ask the snapshot scheduler where it would like to persist states,
    // so that we can later persist them as local snapshots.
    const snapshotPointers = this.snapshotScheduler.getSnapshotLevels(cycleStart, events, limit + 1)
    for (const snapPtr of snapshotPointers) {
      assert(
        snapPtr.i > limit,
        'Scheduler must not ask for snapshots earlier than the given `from`',
      )
      ptrs.push(snapPtr)
    }

    for (let i = events.length - 1; i > limit; i -= 1) {
      const source = events[i].source.sourceId

      if (observedSources.has(source)) {
        continue
      }
      observedSources.add(source)

      const newPtr = {
        tag: source,
        i,
        persistAsLocalSnapshot: false,
      }
      ptrs.push(newPtr)

      // All other cached states are disabled for now due to potential excessive memory+cpu usage
      if (observedSources.size > 1 && !this.expensive) {
        return ptrs.sort(TaggedIndex.ord.compare)
      }
    }

    ptrs.push(...this.assignRecent(cycleStart, events.length, limit))

    // We could improve this by merge-sorting all three strategies’ results.
    const newPointersAscending = ptrs.sort(TaggedIndex.ord.compare)
    return newPointersAscending
  }

  public getSnapshotsToPersist(): StatePointer<S, E>[] {
    return Object.values(this.pendingApplication).sort(TaggedIndex.ord.compare)
  }

  /**
   * Shift all state’s indices back by some amount. To be used when the corresponding event buffer
   * is truncated at the front
   *
   * @param newPointers The list of new pointers that were populated, probably due to us having
   * requested them via return value of `getPointersToStore`.
   */
  public addPopulatedPointers(newPointers: StatePointer<S, E>[], tip: EnvelopeFromStore<E>): void {
    if (newPointers.length === 0) {
      return
    }

    // Since the final event is always the tip of some source,
    // our latest pointer should always be built on the final event.
    this.latest = last(newPointers)

    // We have progressed in time and some of the local snapshots queued
    // by the scheduler in the past may now be eligible for application to the FES.
    this.migrateQueuedSnapshots(tip)

    // Integrate the new states
    for (const ptr of newPointers) {
      if (ptr.persistAsLocalSnapshot) {
        this.enqueueLocalSnapshot(ptr, tip)
      } else {
        // We must not store local snapshots in the main record,
        // else we will get totally confused about the updates to `i`.
        this.ephemeral[ptr.tag] = ptr
      }
    }
  }

  // Just a shortcut
  private eligible(pendingSnap: StatePointer<S, E>, tip: EnvelopeFromStore<E>): boolean {
    return this.snapshotScheduler.isEligibleForStorage(pendingSnap.finalIncludedEvent, tip)
  }

  // Qualify local snapshots that were enqueued in the past, to be persisted now.
  private migrateQueuedSnapshots(tip: EnvelopeFromStore<E>): void {
    const pendingTags = Object.keys(this.pendingEligibility)

    for (const tag of pendingTags) {
      const snap = this.pendingEligibility[tag]

      if (this.eligible(snap, tip)) {
        delete this.pendingEligibility[tag]
        this.pendingApplication[tag] = snap
      }
    }
  }

  private enqueueLocalSnapshot(snap: StatePointer<S, E>, tip: EnvelopeFromStore<E>): void {
    const applyImmediately = this.eligible(snap, tip)

    if (applyImmediately) {
      this.pendingApplication[snap.tag] = snap
    } else {
      this.pendingEligibility[snap.tag] = snap
    }
  }
}
