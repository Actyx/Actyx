/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable, none, Option, some } from 'fp-ts/lib/Option'
import { greaterThan } from 'fp-ts/lib/Ord'
import { Observable } from 'rxjs'
import { EventStore } from '../eventstore'
import {
  AllEventsSortOrders,
  Event,
  Events,
  OffsetMap,
  PersistedEventsSortOrders,
} from '../eventstore/types'
import log from '../loggers'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import { EventKey, FishId, LocalSnapshot } from '../types'
import { runStats, takeWhileInclusive } from '../util'
import { getInsertionIndex } from '../util/binarySearch'

// New API:
// Stream events as they become available, until time-travel would occour.
// To be eventually implemented on the rust-store side with lots of added cleverness.

export enum MsgType {
  state = 'state',
  events = 'events',
  timetravel = 'timetravel',
}

export type StateMsg = Readonly<{
  type: MsgType.state
  snapshot: LocalSnapshot<string>
}>

export type EventsMsg = Readonly<{
  type: MsgType.events
  events: Events
  caughtUp: boolean
}>

export type TimetravelMsg = Readonly<{
  type: MsgType.timetravel
  trigger: Event // earliest known event to cause time travel
  high: Event // latest known event to cause time travel
}>

export type EventsOrTimetravel = StateMsg | EventsMsg | TimetravelMsg

export type SubscribeMonotonic = (
  fishId: FishId,
  subscriptions: SubscriptionSet,
  // Sending 'from' means we DONT want a snapshot
  from?: OffsetMap,
  horizon?: EventKey,
) => Observable<EventsOrTimetravel>

const eventKeyGreater = greaterThan(EventKey.ord)

/**
 * Create a new endpoint, based on the given EventStore and SnapshotStore.
 * The returned function itself is stateless between subsequent calls --
 * all state is within the EventStore itself.
 */
export const eventsMonotonic = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
): SubscribeMonotonic => {
  // Stream realtime events from the given point on.
  // As soon as time-travel would occur, the stream terminates with a TimetravelMsg.
  const realtimeFrom = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    startFrom: OffsetMap,
    knownLatest: EventKey,
  ): Observable<EventsOrTimetravel> => {
    const realtimeEvents = eventStore.allEvents(
      {
        psns: startFrom,
        default: 'min',
      },
      { psns: {}, default: 'max' },
      subscriptions,
      AllEventsSortOrders.Unsorted,
      undefined, // Horizon handling to-be-implemented
    )

    let latest = knownLatest

    return realtimeEvents
      .filter(next => next.length > 0)
      .mergeMap<Events, EventsOrTimetravel>(nextUnsorted => {
        // Delivered chunks are potentially not sorted
        const next = [...nextUnsorted].sort(EventKey.ord.compare)

        // Take while we are going strictly forwards
        const nextKey = next[0]
        const nextIsOlderThanLatest = eventKeyGreater(latest, nextKey)

        if (nextIsOlderThanLatest) {
          return Observable.from(
            snapshotStore
              .invalidateSnapshots(fishId.entityType, fishId.name, nextKey)
              .then(() => timeTravelMsg(latest, next)),
          )
        }

        log.pond.debug('rt passed, ' + JSON.stringify(nextKey) + ' > ' + JSON.stringify(latest))

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

  // The only reason we need this step is that allEvents makes no effort whatsoever
  // to give you a proper ordering for *known* events; so we must take care of it by first streaming *to* present.

  // Stream events monotonically from the given point on.
  // This function is needed, because `realtimeFrom` will return *past* data out of order, too.
  // So in order to have a meaningful shot at reaching a stable state, we must first "forward-stream" up to the known present,
  // and then switch over to "realtime" streaming.
  const monotonicFrom = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    present: OffsetMap,
    lowerBound: OffsetMap = {},
    defaultLatest: EventKey = EventKey.zero,
  ): Observable<EventsOrTimetravel> => {
    const persisted = eventStore
      .persistedEvents(
        { default: 'min', psns: lowerBound },
        { default: 'min', psns: present },
        subscriptions,
        PersistedEventsSortOrders.EventKey,
        undefined, // Horizon handling to-be-implemented
      )
      // Past events are loaded all in one chunk -- FIXME.
      .toArray()

    return persisted.concatMap(chunks => {
      // flatten
      const events = new Array<Event>().concat(...chunks)

      const latest = events.length === 0 ? defaultLatest : events[events.length - 1]

      const initial = Observable.of<EventsMsg>({
        type: MsgType.events,
        events,
        caughtUp: true,
      })

      return initial.concat(realtimeFrom(fishId, subscriptions, present, latest))
    })
  }

  const tryReadSnapshot = (fishId: FishId): Observable<Option<LocalSnapshot<string>>> => {
    const semantics = fishId.entityType
    const name = fishId.name
    const version = fishId.version

    return Observable.from(snapshotStore.retrieveSnapshot(semantics, name, version)).map(x => {
      runStats.counters.add(`snapshot-wanted/${semantics}`)
      return fromNullable(x).fold(none, localSnapshot => {
        runStats.counters.add(`snapshot-found/${semantics}`)
        return some(localSnapshot)
      })
    })
  }

  const startFromSnapshot = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    present: OffsetMap,
  ) => (snap: LocalSnapshot<string>) => {
    const earliestNewEvent = eventStore
      .persistedEvents(
        { default: 'min', psns: snap.psnMap },
        { default: 'min', psns: present },
        subscriptions,
        PersistedEventsSortOrders.EventKey,
        snap.horizon,
      )
      .defaultIfEmpty([])
      .take(1)

    return earliestNewEvent.concatMap(earliest => {
      const snapshotOutdated =
        earliest.length > 0 && EventKey.ord.compare(earliest[0], snap.eventKey) < 0

      if (snapshotOutdated) {
        // Invalidate this snapshot and try again.
        return Observable.from(
          snapshotStore.invalidateSnapshots(fishId.entityType, fishId.name, earliest[0]),
        )
          .take(1)
          .concatMap(() => observeMonotonicFromSnapshot(fishId, subscriptions))
      }

      // Otherwise just pick up from snapshot
      return Observable.concat(
        Observable.of(stateMsg(snap)),
        monotonicFrom(fishId, subscriptions, present, snap.psnMap, snap.eventKey),
      )
    })
  }

  const observeMonotonicFromSnapshot = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    _horizon?: EventKey,
  ): Observable<EventsOrTimetravel> => {
    return Observable.combineLatest(
      tryReadSnapshot(fishId).take(1),
      eventStore.present().take(1),
    ).concatMap(([maybeSnapshot, present]) =>
      maybeSnapshot.fold(
        // No snapshot found -> start from scratch
        monotonicFrom(fishId, subscriptions, present.psns),
        startFromSnapshot(fishId, subscriptions, present.psns),
      ),
    )
  }

  return (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    from?: OffsetMap,
    _horizon?: EventKey,
  ): Observable<EventsOrTimetravel> => {
    if (from) {
      // Client explicitly requests us to start at a certain point
      return eventStore
        .present()
        .first()
        .concatMap(present => monotonicFrom(fishId, subscriptions, present.psns, from))
    } else {
      // `from` NOT given -> try finding a snapshot
      return observeMonotonicFromSnapshot(fishId, subscriptions)
    }
  }
}

const stateMsg = (snapshot: LocalSnapshot<string>): StateMsg => {
  log.pond.info('picking up from local snapshot ' + EventKey.format(snapshot.eventKey))

  return {
    type: MsgType.state,
    snapshot,
  }
}

const timeTravelMsg = (previousHead: EventKey, next: Events): TimetravelMsg => {
  log.pond.info('triggered time-travel back to ' + EventKey.format(next[0]))

  const high = getInsertionIndex(next, previousHead, EventKey.ord.compare) - 1

  return {
    type: MsgType.timetravel,
    trigger: next[0],
    high: next[high],
  }
}
