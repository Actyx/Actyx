/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable, none, Option, some } from 'fp-ts/lib/Option'
import { flatten } from 'ramda'
import { Observable } from 'rxjs'
import { EventStore } from '../eventstore'
import {
  AllEventsSortOrders,
  Event,
  Events,
  OffsetMap,
  OffsetMapWithDefault,
  PersistedEventsSortOrders,
} from '../eventstore/types'
import log from '../loggers'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import { EventKey, FishId, LocalSnapshot } from '../types'
import { runStats, takeWhileInclusive } from '../util'
import { getInsertionIndex } from '../util/binarySearch'

export enum MsgType {
  state = 'state',
  events = 'events',
  timetravel = 'timetravel',
}

export type StateMsg = {
  type: MsgType.state
  snapshot: LocalSnapshot<string>
}

export type EventsMsg = {
  type: MsgType.events
  events: Events
  caughtUp: boolean
}

export type TimetravelMsg = {
  type: MsgType.timetravel
  trigger: Event
  high: Event
}

export type EventsOrTimetravel = StateMsg | EventsMsg | TimetravelMsg

export type SubscribeMonotonic = (
  fishId: FishId,
  subscriptions: SubscriptionSet,
  // Sending 'from' means we DONT want a snapshot
  from?: OffsetMap,
  horizon?: EventKey,
) => Observable<EventsOrTimetravel>

// New API:
// Stream events as they become available, until time-travel would occour.
// To be eventually implemented on the rust-store side with lots of added cleverness.
export const eventsMonotonic: (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
) => SubscribeMonotonic = (eventStore, snapshotStore) => {
  const realtimeFrom = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    startFrom: OffsetMap,
    latest: EventKey,
  ): Observable<EventsOrTimetravel> => {
    const realtimeEvents = eventStore.allEvents(
      {
        psns: startFrom,
        default: 'min',
      },
      { psns: {}, default: 'max' },
      subscriptions,
      AllEventsSortOrders.Unsorted,
      undefined, // horizon,
    )

    return realtimeEvents
      .filter(next => next.length > 0)
      .mergeMap(
        (nextUnsorted): Observable<EventsOrTimetravel> => {
          const next = [...nextUnsorted].sort(EventKey.ord.compare)

          // Take while we are going strictly forwards
          const nextKey = next[0]
          const pass = EventKey.ord.compare(nextKey, latest) >= 0

          if (!pass) {
            return Observable.from(
              snapshotStore
                .invalidateSnapshots(fishId.entityType, fishId.name, nextKey)
                .then(() => timeTravelMsg(latest, next)),
            )
          }

          log.pond.info('rt passed, ' + JSON.stringify(nextKey) + ' > ' + JSON.stringify(latest))

          latest = next[next.length - 1]
          const r: EventsMsg = {
            type: MsgType.events,
            events: next,
            caughtUp: true,
          }
          return Observable.of(r)
        },
      )
      .pipe(takeWhileInclusive(m => m.type !== MsgType.timetravel))
  }

  // The only reason we need this step is that allEvents will make no effort whatsoever
  // to give you a proper ordering for *known* events; so we must take care of it by first streaming *to* present.
  const monotonicFrom = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    present: OffsetMap,
    lowerBound: OffsetMap = {},
  ): Observable<EventsOrTimetravel> => {
    const persisted = eventStore
      .persistedEvents(
        { default: 'min', psns: lowerBound },
        { default: 'min', psns: present },
        subscriptions,
        PersistedEventsSortOrders.EventKey,
        undefined, // No semantic snapshots means no horizon, ever.
      )
      .toArray()

    return persisted.concatMap(chunks => {
      const events = flatten(chunks)

      const latest = events.length === 0 ? EventKey.zero : events[events.length - 1]

      const initial = Observable.of<EventsMsg>({
        type: MsgType.events,
        events: flatten(chunks),
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

  const startFromMaybeSnapshot = (fishId: FishId, subscriptions: SubscriptionSet) => ([
    maybeSnapshot,
    present,
  ]: [Option<LocalSnapshot<string>>, OffsetMapWithDefault]) =>
    maybeSnapshot.fold(
      // No snapshot found-> start from scratch
      monotonicFrom(fishId, subscriptions, present.psns),
      snap => {
        const earliestNewEvent = eventStore
          .persistedEvents(
            { default: 'min', psns: snap.psnMap },
            { default: 'min', psns: present.psns },
            subscriptions,
            PersistedEventsSortOrders.EventKey,
            undefined, // No semantic snapshots means no horizon, ever.
          )
          .defaultIfEmpty([])
          .take(1)

        return earliestNewEvent.concatMap(earliest => {
          // Snapshot is already outdated -> try again
          if (earliest.length > 0 && EventKey.ord.compare(earliest[0], snap.eventKey) < 0) {
            return Observable.from(
              snapshotStore.invalidateSnapshots(fishId.entityType, fishId.name, earliest[0]),
            )
              .take(1)
              .concatMap(() => observeMonotonicFromSnapshot(fishId, subscriptions))
          }

          // Otherwise just pick up from snapshot
          return Observable.concat(
            Observable.of(stateMsg(snap)),
            monotonicFrom(fishId, subscriptions, present.psns, snap.psnMap),
          )
        })
      },
    )

  const observeMonotonicFromSnapshot = (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    _horizon?: EventKey,
  ): Observable<EventsOrTimetravel> => {
    return Observable.combineLatest(
      tryReadSnapshot(fishId).take(1),
      eventStore.present().take(1),
    ).concatMap(startFromMaybeSnapshot(fishId, subscriptions))
  }

  return (
    fishId: FishId,
    subscriptions: SubscriptionSet,
    from?: OffsetMap,
    _horizon?: EventKey,
  ): Observable<EventsOrTimetravel> => {
    // `from` NOT given -> try finding a snapshot
    if (from) {
      // Client explicitly requests us to start at a certain point
      return eventStore
        .present()
        .take(1)
        .concatMap(present => monotonicFrom(fishId, subscriptions, present.psns, from))
    } else {
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

  const high = getInsertionIndex(next, previousHead, (e, l) => EventKey.ord.compare(e, l)) - 1

  return {
    type: MsgType.timetravel,
    trigger: next[0],
    high: next[high], // highest event to cause time-travel
  }
}
