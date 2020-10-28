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
      .map(nextUnsorted => {
        const next = [...nextUnsorted].sort(EventKey.ord.compare)

        // Take while we are going strictly forwards
        const nextKey = next[0]
        const pass = EventKey.ord.compare(nextKey, latest) >= 0

        if (!pass) {
          // FIXME: Invalidate snapshots
          return timeTravelMsg(latest, next)
        }

        log.pond.info('rt passed, ' + JSON.stringify(nextKey) + ' > ' + JSON.stringify(latest))

        latest = next[next.length - 1]
        const r: EventsMsg = {
          type: MsgType.events,
          events: next,
          caughtUp: true,
        }
        return r
      })
      .pipe(takeWhileInclusive(m => m.type !== MsgType.timetravel))
  }

  const monotonicFrom = (
    subscriptions: SubscriptionSet,
    present: OffsetMap,
    lowerBound?: OffsetMap,
  ): Observable<EventsOrTimetravel> => {
    const persisted = eventStore
      .persistedEvents(
        { default: 'min', psns: lowerBound || {} },
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

      return initial.concat(realtimeFrom(subscriptions, present, latest))
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

  const startFromMaybeSnapshot = (subscriptions: SubscriptionSet) => ([maybeSnapshot, present]: [
    Option<LocalSnapshot<string>>,
    OffsetMapWithDefault
  ]) =>
    maybeSnapshot.fold(
      // No snapshot -> start from scratch
      monotonicFrom(subscriptions, present.psns),
      x =>
        Observable.concat(
          Observable.of(stateMsg(x)),
          monotonicFrom(subscriptions, present.psns, x.psnMap),
        ),
    )

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
        .concatMap(present => monotonicFrom(subscriptions, present.psns, from))
    } else {
      return Observable.combineLatest(
        tryReadSnapshot(fishId).take(1),
        eventStore.present().take(1),
      ).concatMap(startFromMaybeSnapshot(subscriptions))
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
