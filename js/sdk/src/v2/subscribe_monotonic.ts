/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable, Option } from 'fp-ts/lib/Option'
import { greaterThan } from 'fp-ts/lib/Ord'
import { Observable } from 'rxjs'
import { SnapshotStore } from '../snapshotStore'
import { EventKey, FixedStart, MsgType, OffsetMap, SerializedStateSnap, Where } from '../types'
import { runStats, takeWhileInclusive } from '../util'
import { getInsertionIndex } from '../util/binarySearch'
import { EventStore } from './eventStore'
import log from './log'
import { AllEventsSortOrders, Event, Events, PersistedEventsSortOrders } from './types'

// New API:
// Stream events as they become available, until time-travel would occour.
// To be eventually implemented on the rust-store side with lots of added cleverness.

export type StateMsg = Readonly<{
  type: MsgType.state
  snapshot: SerializedStateSnap
}>

export type EventsMsg = Readonly<{
  type: MsgType.events
  events: Events
  caughtUp: boolean
}>

export type TimeTravelMsg = Readonly<{
  type: MsgType.timetravel
  trigger: Event // earliest known event to cause time travel
  high: Event // latest known event to cause time travel
}>

export type EventsOrTimetravel = StateMsg | EventsMsg | TimeTravelMsg

export type SubscribeMonotonic = (
  fishId: SessionId,
  subscriptions: Where<unknown>,
  // Sending 'attemptStartFrom' means we DONT want a snapshot
  attemptStartFrom?: FixedStart,
) => Observable<EventsOrTimetravel>

const eventKeyGreater = greaterThan(EventKey.ord)

type SessionId = string
const GenericSemantics = 'generic-snapshot-v2'

/**
 * Create a new endpoint, based on the given EventStore and SnapshotStore.
 * The returned function itself is stateless between subsequent calls --
 * all state is within the EventStore itself.
 */
export const eventsMonotonic = (
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
    const realtimeEvents = eventStore.allEvents(
      {
        psns: fixedStart.from,
        default: 'min',
      },
      { psns: {}, default: 'max' },
      subscriptions,
      AllEventsSortOrders.Unsorted,
      fixedStart.horizon,
    )

    let latest = fixedStart.latestEventKey

    return realtimeEvents
      .filter(next => next.length > 0)
      .mergeMap<Events, EventsOrTimetravel>(nextUnsorted => {
        // Delivered chunks are potentially not sorted
        const next = [...nextUnsorted].sort(EventKey.ord.compare)

        // Take while we are going strictly forwards
        const nextKey = next[0]
        const nextIsOlderThanLatest = eventKeyGreater(latest, nextKey)

        if (nextIsOlderThanLatest) {
          log.submono.debug(
            'started from',
            fixedStart.from,
            'got triggered by stream',
            nextKey.stream,
            'offset',
            nextKey.offset,
          )

          return Observable.from(
            snapshotStore
              .invalidateSnapshots(GenericSemantics, fishId, nextKey)
              .then(() => timeTravelMsg(fishId, latest, next)),
          )
        }

        log.submono.debug(
          'order-check passed: ' + EventKey.format(nextKey) + ' > ' + EventKey.format(latest),
          'for realtime chunk of size',
          next.length,
        )

        // We have captured `latest` in the closure and are updating it here
        latest = next[next.length - 1]
        return Observable.of({
          type: MsgType.events,
          events: next,
          caughtUp: true,
        })
      })
      .pipe(takeWhileInclusive(m => m.type !== MsgType.timetravel))
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

    const persisted: Observable<EventsMsg> = eventStore
      .persistedEvents(
        { default: 'min', psns: fixedStart.from },
        { default: 'min', psns: present },
        subscriptions,
        PersistedEventsSortOrders.Ascending,
        fixedStart.horizon,
      )
      .filter(chunk => chunk.length > 0)
      .do(chunk => (latest = chunk[chunk.length - 1]))
      .map(chunk => ({
        type: MsgType.events,
        events: chunk,
        caughtUp: false,
      }))

    const realtimeStream = Observable.defer(() =>
      realtimeFrom(fishId, subscriptions, {
        from: present,
        latestEventKey: latest,
        horizon: fixedStart.horizon,
      }),
    )

    return Observable.concat(
      persisted,
      Observable.of<EventsMsg>({
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
    whenInvalid: (outdatedChunk: Events) => Observable<EventsOrTimetravel>,
    whenValid: () => Observable<EventsOrTimetravel>,
  ): Observable<EventsOrTimetravel> => {
    const earliestNewEvents = eventStore
      .persistedEvents(
        { default: 'min', psns: attemptStartFrom.from },
        { default: 'min', psns: present },
        subscriptions,
        PersistedEventsSortOrders.Ascending,
        attemptStartFrom.horizon,
      )
      // testEventStore can send empty chunks, real store hopefully will not
      .filter(chunk => chunk.length > 0)
      .defaultIfEmpty([])
      .first()

    // Find the earliest persistent chunk after the starting point and see whether it’s after the FixedStart
    return earliestNewEvents.concatMap(earliest => {
      const offsetOutdated =
        earliest.length > 0 && eventKeyGreater(attemptStartFrom.latestEventKey, earliest[0])

      return offsetOutdated ? whenInvalid(earliest) : whenValid()
    })
  }

  // Client thinks it has valid offsets to start from -- it may be wrong, though!
  const startFromFixedOffsets = (
    fishId: SessionId,
    subscriptions: Where<unknown>,
    present: OffsetMap,
  ) => (attemptStartFrom: FixedStart): Observable<EventsOrTimetravel> => {
    const whenValid = () => monotonicFrom(fishId, subscriptions, present, attemptStartFrom)

    const whenInvalid = (earliest: Events) => {
      log.submono.debug(
        fishId,
        'discarding outdated requested FixedStart',
        EventKey.format(attemptStartFrom.latestEventKey),
        'due to',
        EventKey.format(earliest[0]),
      )

      return Observable.of(timeTravelMsg(fishId, attemptStartFrom.latestEventKey, earliest))
    }

    return validateFixedStart(subscriptions, present, attemptStartFrom, whenInvalid, whenValid)
  }

  const tryReadSnapshot = async (fishId: SessionId): Promise<Option<SerializedStateSnap>> => {
    const retrieved = await snapshotStore.retrieveSnapshot(GenericSemantics, fishId, 0)

    runStats.counters.add(`snapshot-wanted/${fishId}`)
    return fromNullable(retrieved).map(x => {
      runStats.counters.add(`snapshot-found/${fishId}`)
      return x
    })
  }

  // Try start from a snapshot we have found. The snapshot may be outdated, though.
  const startFromSnapshot = (
    fishId: SessionId,
    subscriptions: Where<unknown>,
    present: OffsetMap,
  ) => (snap: SerializedStateSnap): Observable<EventsOrTimetravel> => {
    const fixedStart = {
      from: snap.offsets,
      horizon: snap.horizon,
      latestEventKey: snap.eventKey,
    }

    const whenInvalid = (earliest: Events) => {
      log.submono.debug(
        fishId,
        'discarding outdated snapshot',
        EventKey.format(snap.eventKey),
        'due to',
        EventKey.format(earliest[0]),
      )

      return Observable.from(
        snapshotStore.invalidateSnapshots('generic-snapshot-v2', fishId, earliest[0]),
      )
        .first()
        .concatMap(() => observeMonotonicFromSnapshot(fishId, subscriptions))
    }

    const whenValid = () =>
      Observable.concat(
        Observable.of(stateMsg(fishId, snap)),
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
    return Observable.combineLatest(
      Observable.from(tryReadSnapshot(fishId)).first(),
      Observable.from(eventStore.offsets()).map(({ present }) => present),
    ).concatMap(([maybeSnapshot, present]) =>
      maybeSnapshot.fold(
        // No snapshot found -> start from scratch
        monotonicFrom(fishId, subscriptions, present),
        startFromSnapshot(fishId, subscriptions, present),
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
      return Observable.from(eventStore.offsets()).concatMap(offsets =>
        startFromFixedOffsets(fishId, subscriptions, offsets.present)(attemptStartFrom),
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
