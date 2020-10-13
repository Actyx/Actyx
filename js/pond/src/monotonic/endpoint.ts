/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any @typescript-eslint/no-let */
import { Observable } from 'rxjs'
import { EventStore } from '../eventstore'
import { AllEventsSortOrders, Event, Events, OffsetMap, OffsetMapWithDefault, PersistedEventsSortOrders } from '../eventstore/types'
import log from '../loggers'
import { SubscriptionSet } from '../subscription'
import { EventKey, LocalSnapshot } from '../types'
import { takeWhileInclusive } from '../util'
import { getInsertionIndex } from '../util/binarySearch'


export enum MsgType {
    state = 'state',
    events = 'events',
    timetravel = 'timetravel',
}

export type StateMsg = {
    type: MsgType.state
    state: LocalSnapshot<unknown>
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

// New API:
// Stream events as they become available, until time-travel would occour.
// To be implemented on the store side with lots of cleverness.
// - Should do its best to supply events in forward-order (currently not true for Ruststore allEvents API)
// - Should buffer recent X events like the FES used to, for quick travel to the recent past
// - Should quickly find `from`
// - TODO: Should signal when pond can start to emit states (no pending events on store side)
// - etc.
export const eventsMonotonic = (eventStore: EventStore) => (
    subscriptions: SubscriptionSet,
    // Sending 'from' means we dont want a snapshot
    _from?: OffsetMap,
    _horizon?: EventKey,
): Observable<EventsOrTimetravel> => {
    return eventStore
        .present()
        .take(1)
        .concatMap(present => monotonicFrom(eventStore, subscriptions, present))
}

const monotonicFrom = (eventStore: EventStore, subscriptions: SubscriptionSet,
    present: OffsetMapWithDefault,
): Observable<EventsOrTimetravel> => {


    const persisted: Observable<EventsMsg> = eventStore.persistedEvents(
        { default: 'min', psns: {} },
        { default: 'min', psns: present.psns },
        subscriptions,
        PersistedEventsSortOrders.EventKey,
        undefined, // No semantic snapshots means no horizon, ever.
    ).map(chunk => ({
        type: MsgType.events,
        events: chunk,
        caughtUp: false
    }))

    const realtime = Observable.defer(() => realtimeFrom(eventStore, subscriptions, present))

    return persisted.concat(realtime)
}

const realtimeFrom = (eventStore: EventStore, subscriptions: SubscriptionSet,
    present: OffsetMapWithDefault,
): Observable<EventsOrTimetravel> => {

    let latest = EventKey.zero

    const realtimeEvents = eventStore
        .allEvents(
            {
                psns: present.psns,
                default: 'min',
            },
            { psns: {}, default: 'max' },
            subscriptions,
            AllEventsSortOrders.Unsorted,
            undefined // horizon,
        )
    // Buffer incoming updates, try to preserve forward order -- act like a clever rust-side!
    // .bufferTime(3)
    // .map(chunks => {
    //     const flattened: Event[] = chunks.reduce((chunk: Events, acc: Events) => acc.concat(chunk), [] as Event[])
    //     return flattened.sort(EventKey.ord.compare)
    // }
    // ))

    return realtimeEvents
        .filter(next => next.length > 0)
        .map(next => {
            // Take while we are going strictly forwards
            const nextKey = next[0]
            const pass = EventKey.ord.compare(nextKey, latest) >= 0

            if (!pass) {
                log.pond.info('triggered time-travel back to ' + JSON.stringify(nextKey))

                const high =
                    getInsertionIndex(next, latest, (e, l) =>
                        EventKey.ord.compare(e, l),
                    ) - 1

                return {
                    type: MsgType.timetravel,
                    trigger: next[0],
                    high: next[high], // highest event to cause time-travel
                } as TimetravelMsg
            }

            log.pond.info('rt passed, ' + JSON.stringify(nextKey) + ' > ' + JSON.stringify(latest))

            latest = next[next.length - 1]
            return {
                type: MsgType.events,
                events: next,
            } as EventsMsg
        })
        .pipe(takeWhileInclusive(m => m.type !== MsgType.timetravel))
}
