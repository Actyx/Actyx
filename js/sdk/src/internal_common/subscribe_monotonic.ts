/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { fromNullable, Option, map as mapOption, fold as foldOption } from 'fp-ts/lib/Option'
import { gt } from 'fp-ts/lib/Ord'
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
  trigger: EventKey
}

export type EventsOrTimetravel = StateMsg | EventsMsg | TimeTravelMsg

export type SubscribeMonotonic = (
  session: string,
  subscriptions: Where<unknown>,
  attemptStartFrom: FixedStart,
) => Observable<EventsOrTimetravel>

const eventKeyGreater = gt(EventKey.ord)

const horizonFilter = (fixedStart: FixedStart | undefined) =>
  fixedStart?.horizon ? `${fixedStart.horizon.lamport}/${fixedStart.horizon.stream}` : undefined

/**
 * Create a new endpoint, based on the given EventStore and SnapshotStore.
 * The returned function itself is stateless between subsequent calls --
 * all state is within the EventStore itself.
 */
export const eventsMonotonic = (eventStore: EventStore): SubscribeMonotonic => {
  // Stream realtime events from the given point on.
  // Actyx delivers events before `present` (i.e. before the first `offsets` msg) in ascending order.
  // As soon as time-travel would occur, the stream terminates with a TimetravelMsg.
  const monotonicFrom = (
    session: string,
    subscriptions: Where<unknown>,
    fixedStart?: FixedStart,
  ): Observable<EventsOrTimetravel> => {
    const horizon = horizonFilter(fixedStart)
    let diagPrinted = false
    return eventStore
      .subscribeMonotonic(session, fixedStart?.from || {}, subscriptions, horizon)
      .pipe(
        bufferOp(1),
        concatMap((next) => {
          const emit: EventsOrTimetravel[] = []
          let events: EventsMsg | null = null
          for (const x of next) {
            switch (x.type) {
              case 'diagnostic': {
                if (!diagPrinted) {
                  diagPrinted = true
                  log.submono.debug(`(${session}) AQL ${x.severity}: ${x.message}`)
                }
                break
              }
              case 'event': {
                x.caughtUp && log.submono.debug('caught up', session)
                if (events === null) {
                  events = { type: MsgType.events, events: [x], caughtUp: x.caughtUp }
                  emit.push(events)
                } else {
                  events.events.push(x)
                  events.caughtUp = x.caughtUp
                }
                break
              }
              case 'offsets': {
                if (events === null) {
                  events = { type: MsgType.events, events: [], caughtUp: true }
                  emit.push(events)
                } else {
                  events.caughtUp = true
                }
                break
              }
              case 'timeTravel': {
                events = null
                emit.push({ type: MsgType.timetravel, trigger: x.newStart })
              }
            }
          }
          return emit
        }),
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
    return eventStore
      .query(
        attemptStartFrom.from,
        present,
        subscriptions,
        EventsSortOrder.Ascending,
        horizonFilter(attemptStartFrom),
      )
      .pipe(
        defaultIfEmpty(null),
        first(),
        concatMap((earliest) =>
          earliest && eventKeyGreater(attemptStartFrom.latestEventKey, earliest)
            ? whenInvalid(earliest)
            : whenValid(),
        ),
      )
  }

  // Client thinks it has valid offsets to start from -- it may be wrong, though!
  const startFromFixedOffsets =
    (session: string, subscriptions: Where<unknown>, present: OffsetMap) =>
    (attemptStartFrom: FixedStart): Observable<EventsOrTimetravel> => {
      const whenValid = () => monotonicFrom(session, subscriptions, attemptStartFrom)

      const whenInvalid = (earliest: Event) => {
        log.submono.debug(
          session,
          'discarding outdated requested FixedStart',
          EventKey.format(attemptStartFrom.latestEventKey),
          'due to',
          EventKey.format(earliest),
        )

        // TODO this time travel msg should also have a good `high` element
        // (consider this if/when ever implementing this Rust-side)
        return of(timeTravelMsg(session, attemptStartFrom.latestEventKey, [earliest]))
      }

      return validateFixedStart(subscriptions, present, attemptStartFrom, whenInvalid, whenValid)
    }

  return (
    session: string,
    subscriptions: Where<unknown>,
    attemptStartFrom: FixedStart,
  ): Observable<EventsOrTimetravel> => {
    // Client explicitly requests us to start at a certain point
    return from(eventStore.offsets()).pipe(
      concatMap((offsets) =>
        startFromFixedOffsets(session, subscriptions, offsets.present)(attemptStartFrom),
      ),
    )
  }
}

const timeTravelMsg = (session: string, previousHead: EventKey, next: Events): TimeTravelMsg => {
  log.submono.info(session, 'must time-travel back to:', EventKey.format(next[0]))

  const high = getInsertionIndex(next, previousHead, EventKey.ord.compare) - 1

  return {
    type: MsgType.timetravel,
    trigger: next[0],
  }
}
