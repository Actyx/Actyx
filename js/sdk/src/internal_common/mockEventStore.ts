/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { ReplaySubject, lastValueFrom, EMPTY, of } from '../../node_modules/rxjs'
import { take, concatMap, tap, map } from '../../node_modules/rxjs/operators'
import { AppId, Lamport, NodeId, Offset, OffsetMap, Timestamp, toEventPredicate } from '../types'
import { DoPersistEvents, DoQuery, DoSubscribe, EventStore } from './eventStore'
import log from './log'
import { Events } from './types'

export const mockEventStore: () => EventStore = () => {
  const nodeId = NodeId.of('MOCK')
  const present = new ReplaySubject<OffsetMap>(1)
  const events = new ReplaySubject<Events>(1e3)
  events.next([])

  let psn = Offset.of(0)
  present.next({})

  const query: DoQuery = (_from, _to, query, __sortOrder) => {
    if (typeof query === 'string') {
      throw new Error('direct AQL not yet supported by mockEventStore')
    }

    return events.pipe(
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore this needs to complete
      take(events._events.length),
      concatMap((x) => x.filter(toEventPredicate(query))),
      tap((x) => log.ws.debug('persistedEvents', x)),
    )
  }

  const subscribe: DoSubscribe = (_from, query) => {
    if (typeof query === 'string') {
      throw new Error('direct AQL not yet supported by mockEventStore')
    }

    return events.asObservable().pipe(
      concatMap((x) => x.filter(toEventPredicate(query))),
      tap((x) => log.ws.debug('allEvents', x)),
    )
  }

  const streamId = NodeId.streamNo(nodeId, 0)

  const persistEvents: DoPersistEvents = (x) => {
    log.ws.debug('putEvents', x)
    const newEvents: Events = x.map((payload) => ({
      payload: payload.payload,
      tags: [],
      appId: AppId.of('test'),
      stream: streamId,
      timestamp: Timestamp.now(),
      lamport: Lamport.of(Timestamp.now()),
      offset: Offset.of(psn++),
    }))

    events.next(newEvents)
    present.next({ [streamId]: psn })
    return of(newEvents)
  }

  const getPresent = () =>
    lastValueFrom(
      present.asObservable().pipe(
        tap(() => log.ws.debug('present')),
        take(1),
        map((present) => ({ present, toReplicate: {} })),
      ),
    )

  return {
    nodeId,
    offsets: getPresent,
    query,
    queryUnchecked: () => {
      throw new Error('not implemented for mock event store')
    },
    subscribe,
    subscribeUnchecked: () => {
      throw new Error('not implemented for mock event store')
    },
    persistEvents,
    connectivityStatus: () => EMPTY,
  }
}
