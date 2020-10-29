/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { greaterThan } from 'fp-ts/lib/Ord'
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
import { SubscriptionSet } from '../subscription'
import { EventKey, LocalSnapshot } from '../types'
import { takeWhileInclusive } from '../util'
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
  state: LocalSnapshot<unknown>
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
  subscriptions: SubscriptionSet,
  // Sending 'from' means we DONT want a snapshot
  _from?: OffsetMap,
  _horizon?: EventKey,
) => Observable<EventsOrTimetravel>

const eventKeyGreater = greaterThan(EventKey.ord)

/**
 * Create a new endpoint, based on the given EventStore.
 * The returned function itself is stateless between subsequent calls --
 * all state is within the EventStore itself.
 */
export const eventsMonotonic = (eventStore: EventStore): SubscribeMonotonic => {
  // Stream realtime events from the given point on.
  // As soon as time-travel would occur, the stream terminates with a TimetravelMsg.
  const realtimeFrom = (
    subscriptions: SubscriptionSet,
    present: OffsetMapWithDefault,
    latest: EventKey,
  ): Observable<EventsOrTimetravel> => {
    const realtimeEvents = eventStore.allEvents(
      {
        psns: present.psns,
        default: 'min',
      },
      { psns: {}, default: 'max' },
      subscriptions,
      AllEventsSortOrders.Unsorted,
      undefined, // Horizon handling to-be-implemented
    )

    return realtimeEvents
      .filter(next => next.length > 0)
      .map<Events, EventsOrTimetravel>(nextUnsorted => {
        // Delivered chunks are potentially not sorted
        const next = [...nextUnsorted].sort(EventKey.ord.compare)

        // Take while we are going strictly forwards
        const nextKey = next[0]
        const nextIsOlderThanLatest = eventKeyGreater(latest, nextKey)

        if (nextIsOlderThanLatest) {
          return timeTravelMsg(latest, next)
        }

        log.pond.debug('rt passed, ' + JSON.stringify(nextKey) + ' > ' + JSON.stringify(latest))

        // We have captured `latest` in the closure and are updating it here
        latest = next[next.length - 1]

        return {
          type: MsgType.events,
          events: next,
          caughtUp: true,
        }
      })
      .pipe(takeWhileInclusive(m => m.type !== MsgType.timetravel))
  }

  // Stream events monotonically from the given point on.
  // This function is needed, because `realtimeFrom` will return *past* data out of order, too.
  // So in order to have a meaningful shot at reaching a stable state, we must first "forward-stream" up to the known present,
  // and then switch over to "realtime" streaming.
  const monotonicFrom = (subscriptions: SubscriptionSet) => (
    present: OffsetMapWithDefault,
  ): Observable<EventsOrTimetravel> => {
    const persisted = eventStore
      .persistedEvents(
        { default: 'min', psns: {} },
        { default: 'min', psns: present.psns },
        subscriptions,
        PersistedEventsSortOrders.EventKey,
        undefined, // Horizon handling to-be-implemented
      )
      // Past events are loaded all in one chunk, this is consistent with FES behavior
      .toArray()

    return persisted.concatMap(chunks => {
      const events = chunks.flat()

      const latest = events.length === 0 ? EventKey.zero : events[events.length - 1]

      const initial = Observable.of<EventsMsg>({
        type: MsgType.events,
        events,
        caughtUp: true,
      })

      return initial.concat(realtimeFrom(subscriptions, present, latest))
    })
  }

  return (
    subscriptions: SubscriptionSet,
    _from?: OffsetMap,
    _horizon?: EventKey,
  ): Observable<EventsOrTimetravel> => {
    return eventStore
      .present()
      .first()
      .concatMap(monotonicFrom(subscriptions))
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
