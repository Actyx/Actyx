/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable, Option, map as mapOption, fold as foldOption } from 'fp-ts/lib/Option'
import { gt, geq } from 'fp-ts/lib/Ord'
import { Observable, EMPTY, from, of, combineLatest, concat, defer } from '../../node_modules/rxjs'
import {
  filter,
  mergeMap,
  concatMap,
  map,
  first,
  defaultIfEmpty,
  bufferCount,
  tap,
} from '../../node_modules/rxjs/operators'
import { LocalSnapshotFromIndex, SnapshotStore } from '../snapshotStore'
import {
  EventKey,
  EventsSortOrder,
  FixedStart,
  MsgType,
  OffsetMap,
  SerializedStateSnap,
  Where,
} from '../types'
import { getInsertionIndex, runStats, takeWhileInclusive } from '../util'
import { EventStore } from './eventStore'
import log from './log'
import { Event, Events } from './types'
import { bufferOp } from '../util/bufferOp'

// New API:
// Stream events as they become available, until time-travel would occour.
// To be eventually implemented on the rust-store side with lots of added cleverness.

export type StateMsg = {
  type: MsgType.state
  snapshot: SerializedStateSnap
}

export type EventsMsg = {
  type: MsgType.events
  events: Events
  caughtUp: boolean
}

export type TimeTravelMsg = {
  type: MsgType.timetravel
  trigger: Event // earliest known event to cause time travel
  high: Event // latest known event to cause time travel
}

export type EventsOrTimetravel = StateMsg | EventsMsg | TimeTravelMsg

export type SubscribeMonotonic = (
  fishId: SessionId,
  subscriptions: Where<unknown>,
  // Sending 'attemptStartFrom' means we DONT want a snapshot
  attemptStartFrom?: FixedStart,
) => Observable<EventsOrTimetravel>

const eventKeyGreater = gt(EventKey.ord)
const eventKeyGreaterEq = geq(EventKey.ord)

type SessionId = string
const GenericSemantics = 'generic-snapshot-v2'

const horizonFilter = (horizon: EventKey) => (x: Event) => eventKeyGreaterEq(x, horizon)

/**
 * Create a new endpoint, based on the given EventStore and SnapshotStore.
 * The returned function itself is stateless between subsequent calls --
 * all state is within the EventStore itself.
 */
export const eventsMonotonicEmulated = (
  eventStore: EventStore,
  // No snapshots are actually available in V2 so far.
  snapshotStore: SnapshotStore,
): SubscribeMonotonic => {
  // Stream realtime events from the given point on.
  // As soon as time-travel would occur, the stream terminates with a TimetravelMsg.
  const realtimeFrom = (
    fishId: SessionId,
    subscriptions: Where<unknown>,
    fixedStart: FixedStart,
  ): Observable<EventsOrTimetravel> => {
    const realtimeEvents = eventStore.subscribe(
      fixedStart.from,
      subscriptions,
      // FIXME: Horizon not supported in V2 yet. https://github.com/Actyx/Cosmos/issues/6730
      // fixedStart.horizon,
    )

    const rtAfterHorizon = fixedStart.horizon
      ? realtimeEvents.pipe(filter(horizonFilter(fixedStart.horizon)))
      : realtimeEvents

    let latest = fixedStart.latestEventKey

    let tt = false

    const liveBuffered = rtAfterHorizon.pipe(
      bufferOp(1),
      filter((x) => x.length > 0),
      mergeMap<Events, Observable<EventsOrTimetravel>>((nextUnsorted) => {
        // Don't spam the logs. And avoid esoteric race conditions due to triggering multiple snapshot invalidations.
        if (tt) {
          return EMPTY
        }

        const next = nextUnsorted.sort(EventKey.ord.compare)

        // Take while we are going strictly forwards
        const nextKey = next[0]
        const nextIsOlderThanLatest = eventKeyGreater(latest, nextKey)

        if (nextIsOlderThanLatest) {
          tt = true

          log.submono.debug(
            'started from',
            fixedStart.from,
            'got triggered by stream',
            nextKey.stream,
            'offset',
            nextKey.offset,
          )

          return from(
            snapshotStore
              .invalidateSnapshots(GenericSemantics, fishId, nextKey)
              .then(() => timeTravelMsg(fishId, latest, next)),
          )
        }

        log.submono.debug(
          'order-check passed: ' + EventKey.format(nextKey) + ' > ' + EventKey.format(latest),
          'for realtime event',
        )

        // We have captured `latest` in the closure and are updating it here
        const newLatest = next[next.length - 1]
        latest = {
          lamport: newLatest.lamport,
          stream: newLatest.stream,
          offset: newLatest.offset,
        }
        return of({
          type: MsgType.events,
          events: next,
          caughtUp: true,
        })
      }),
      // Buffer live events for a small amount of time, so we don’t update state too often.
      // Should be handled by the `caughtUp` flag in the store-side impl.
      takeWhileInclusive((m: EventsOrTimetravel) => m.type !== MsgType.timetravel),
    )

    return liveBuffered
  }

  // The only reason we need the "catch up to present" step is that `allEvents` makes no effort whatsoever
  // to give you a proper ordering for *known* events; so we must take care of it by first streaming *to* present.

  // Stream events monotonically from the given point on.
  // This function is needed, because `realtimeFrom` will return *past* data out of order, too.
  // So in order to have a meaningful shot at reaching a stable state, we must first "forward-stream" up to the known present,
  // and then switch over to "realtime" streaming.
  const monotonicFrom = (
    fishId: SessionId,
    subscriptions: Where<unknown>,
    present: OffsetMap,
    fixedStart: FixedStart = {
      from: {},
      latestEventKey: EventKey.zero,
    },
  ): Observable<EventsOrTimetravel> => {
    let latest = fixedStart.latestEventKey

    const persisted = eventStore.query(
      fixedStart.from,
      present,
      subscriptions,
      EventsSortOrder.Ascending,
      // FIXME: Horizon not supported in V2 yet. https://github.com/Actyx/Cosmos/issues/6730
      // fixedStart.horizon,
    )

    const persistedAfterHorizon = fixedStart.horizon
      ? persisted.pipe(filter(horizonFilter(fixedStart.horizon)))
      : persisted

    const persistedChunked: Observable<EventsMsg> = persistedAfterHorizon
      // Speed up Fish hydration by applying chunks
      .pipe(
        bufferCount(32),
        map<Events, EventsMsg>((chunk) => ({
          type: MsgType.events,
          events: chunk,
          caughtUp: false,
        })),
        tap((msg) => (latest = msg.events[msg.events.length - 1])),
      )

    const realtimeStream = defer(() =>
      realtimeFrom(fishId, subscriptions, {
        from: present,
        latestEventKey: latest,
        horizon: fixedStart.horizon,
      }),
    )

    return concat(
      persistedChunked,
      of<EventsMsg>({
        type: MsgType.events,
        events: [],
        // Empty chunk with caughtUp=true, to trigger emission of current state.
        // The proper impl should obviously set caughtUp=true for the final proper (nonempty) chunk;
        // but we have a hard time detecting the final chunk here.
        caughtUp: true,
      }),
      realtimeStream,
    )
  }

  // Given a FixedStart point, check whether we can reach `present` without time travel.
  // If so, apply whenValid. Otherwise apply whenInvalid to the earliest chunk between start and present.
  const validateFixedStart = (
    subscriptions: Where<unknown>,
    present: OffsetMap,
    attemptStartFrom: FixedStart,
    whenInvalid: (outdatedChunk: Event) => Observable<EventsOrTimetravel>,
    whenValid: () => Observable<EventsOrTimetravel>,
  ): Observable<EventsOrTimetravel> => {
    const earliestNewEvents = eventStore.query(
      attemptStartFrom.from,
      present,
      subscriptions,
      EventsSortOrder.Ascending,
      // FIXME: Horizon not supported in V2 yet. https://github.com/Actyx/Cosmos/issues/6730
      // attemptStartFrom.horizon,
    )
    // FIXME: Store should filter

    const afterHorizon = attemptStartFrom.horizon
      ? earliestNewEvents.pipe(filter(horizonFilter(attemptStartFrom.horizon)))
      : earliestNewEvents

    const earliestNew = afterHorizon.pipe(defaultIfEmpty(null), first())

    // Find the earliest persistent chunk after the starting point and see whether it’s after the FixedStart
    return earliestNew.pipe(
      concatMap((earliest) =>
        earliest && eventKeyGreater(attemptStartFrom.latestEventKey, earliest)
          ? whenInvalid(earliest)
          : whenValid(),
      ),
    )
  }

  // Client thinks it has valid offsets to start from -- it may be wrong, though!
  const startFromFixedOffsets =
    (fishId: SessionId, subscriptions: Where<unknown>, present: OffsetMap) =>
    (attemptStartFrom: FixedStart): Observable<EventsOrTimetravel> => {
      const whenValid = () => monotonicFrom(fishId, subscriptions, present, attemptStartFrom)

      const whenInvalid = (earliest: Event) => {
        log.submono.debug(
          fishId,
          'discarding outdated requested FixedStart',
          EventKey.format(attemptStartFrom.latestEventKey),
          'due to',
          EventKey.format(earliest),
        )

        // TODO this time travel msg should also have a good `high` element
        // (consider this if/when ever implementing this Rust-side)
        return of(timeTravelMsg(fishId, attemptStartFrom.latestEventKey, [earliest]))
      }

      return validateFixedStart(subscriptions, present, attemptStartFrom, whenInvalid, whenValid)
    }

  const tryReadSnapshot = async (fishId: SessionId): Promise<Option<SerializedStateSnap>> => {
    const retrieved = await snapshotStore.retrieveSnapshot(GenericSemantics, fishId, 0)

    runStats.counters.add(`snapshot-wanted/${fishId}`)
    return mapOption((x: LocalSnapshotFromIndex) => {
      runStats.counters.add(`snapshot-found/${fishId}`)
      return x
    })(fromNullable(retrieved))
  }

  // Try start from a snapshot we have found. The snapshot may be outdated, though.
  const startFromSnapshot =
    (fishId: SessionId, subscriptions: Where<unknown>, present: OffsetMap) =>
    (snap: SerializedStateSnap): Observable<EventsOrTimetravel> => {
      const fixedStart = {
        from: snap.offsets,
        horizon: snap.horizon,
        latestEventKey: snap.eventKey,
      }

      const whenInvalid = (earliest: Event) => {
        log.submono.debug(
          fishId,
          'discarding outdated snapshot',
          EventKey.format(snap.eventKey),
          'due to',
          EventKey.format(earliest),
        )

        return from(
          snapshotStore.invalidateSnapshots('generic-snapshot-v2', fishId, earliest),
        ).pipe(
          first(),
          concatMap(() => observeMonotonicFromSnapshot(fishId, subscriptions)),
        )
      }

      const whenValid = () =>
        concat(
          of(stateMsg(fishId, snap)),
          monotonicFrom(fishId, subscriptions, present, {
            from: snap.offsets,
            latestEventKey: snap.eventKey,
            horizon: snap.horizon,
          }),
        )

      return validateFixedStart(subscriptions, present, fixedStart, whenInvalid, whenValid)
    }

  const observeMonotonicFromSnapshot = (
    fishId: SessionId,
    subscriptions: Where<unknown>,
  ): Observable<EventsOrTimetravel> => {
    return combineLatest([
      from(tryReadSnapshot(fishId)).pipe(first()),
      from(eventStore.offsets()).pipe(map(({ present }) => present)),
    ]).pipe(
      concatMap(([maybeSnapshot, present]) =>
        foldOption(
          // No snapshot found -> start from scratch
          () => monotonicFrom(fishId, subscriptions, present),
          startFromSnapshot(fishId, subscriptions, present),
        )(maybeSnapshot),
      ),
    )
  }

  return (
    fishId: SessionId,
    subscriptions: Where<unknown>,
    attemptStartFrom?: FixedStart,
  ): Observable<EventsOrTimetravel> => {
    if (attemptStartFrom) {
      // Client explicitly requests us to start at a certain point
      return from(eventStore.offsets()).pipe(
        concatMap((offsets) =>
          startFromFixedOffsets(fishId, subscriptions, offsets.present)(attemptStartFrom),
        ),
      )
    } else {
      // `from` NOT given -> try finding a snapshot
      return observeMonotonicFromSnapshot(fishId, subscriptions)
    }
  }
}

const stateMsg = (fishId: SessionId, snapshot: SerializedStateSnap): StateMsg => {
  log.submono.info(fishId, 'picking up from local snapshot', EventKey.format(snapshot.eventKey))

  return {
    type: MsgType.state,
    snapshot,
  }
}

const timeTravelMsg = (fishId: SessionId, previousHead: EventKey, next: Events): TimeTravelMsg => {
  log.submono.info(fishId, 'must time-travel back to:', EventKey.format(next[0]))

  const high = getInsertionIndex(next, previousHead, EventKey.ord.compare) - 1

  return {
    type: MsgType.timetravel,
    trigger: next[0],
    high: next[high],
  }
}
