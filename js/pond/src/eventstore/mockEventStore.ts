/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Observable, ReplaySubject } from 'rxjs'
import log from '../store/loggers'
import { toEventPredicate } from '../tagging'
import { Lamport, NodeId, Offset } from '../types'
import {
  EventStore,
  RequestAllEvents,
  RequestPersistedEvents,
  RequestPersistEvents,
} from './eventStore'
import { ConnectivityStatus, Events, OffsetMapWithDefault } from './types'

export const mockEventStore: () => EventStore = () => {
  const nodeId = NodeId.of('MOCK')
  const present = new ReplaySubject<OffsetMapWithDefault>(1)
  const events = new ReplaySubject<Events>(1e3)
  events.next([])

  let psn = Offset.of(0)
  present.next({ psns: {}, default: 'max' })

  const persistedEvents: RequestPersistedEvents = (_from, _to, ss, _min, _sortOrder) => {
    return (
      events
        // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
        // @ts-ignore this needs to complete
        .take(events._events.length)
        .map(x => x.filter(toEventPredicate(ss)))
        .do(x => log.ws.debug('persistedEvents', x))
    )
  }

  const allEvents: RequestAllEvents = (_from, _to, ss, _min, _sortOrder) => {
    return events
      .asObservable()
      .map(x => x.filter(toEventPredicate(ss)))
      .do(x => log.ws.debug('allEvents', x))
  }

  const streamId = NodeId.streamNo(nodeId, 0)

  const persistEvents: RequestPersistEvents = x => {
    log.ws.debug('putEvents', x)
    const newEvents: Events = x.map(payload => ({
      payload: payload.payload,
      tags: [],
      stream: streamId,
      timestamp: payload.timestamp,
      lamport: Lamport.of(payload.timestamp),
      offset: Offset.of(psn++),
    }))

    events.next(newEvents)
    present.next({ psns: { [streamId]: psn }, default: 'max' })
    return Observable.of(newEvents)
  }
  return {
    nodeId,
    present: () => present.asObservable().do(() => log.ws.debug('present')),
    highestSeen: () => present.asObservable(),
    persistedEvents,
    allEvents,
    persistEvents,
    connectivityStatus: () => Observable.empty<ConnectivityStatus>(),
  }
}
