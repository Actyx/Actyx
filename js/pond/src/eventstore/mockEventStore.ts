import { Observable, ReplaySubject } from 'rxjs'
import log from '../store/loggers'
import { subscriptionsToEventPredicate } from '../subscription'
import { Lamport, Psn, SourceId } from '../types'
import {
  EventStore,
  RequestAllEvents,
  RequestPersistedEvents,
  RequestPersistEvents,
} from './eventStore'
import { Events, OffsetMapWithDefault, ConnectivityStatus } from './types'

export const mockEventStore: () => EventStore = () => {
  const sourceId = SourceId.of('MOCK')
  const present = new ReplaySubject<OffsetMapWithDefault>(1)
  const events = new ReplaySubject<Events>(1e3)
  events.next([])

  let psn = Psn.of(0)
  present.next({ psns: {}, default: 'max' })

  const persistedEvents: RequestPersistedEvents = (_from, _to, ss, _min, _sortOrder) => {
    return (
      events
        // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
        // @ts-ignore this needs to complete
        .take(events._events.length)
        .map(x => x.filter(subscriptionsToEventPredicate(ss)))
        .do(x => log.ws.debug('persistedEvents', x))
    )
  }

  const allEvents: RequestAllEvents = (_from, _to, ss, _min, _sortOrder) => {
    return events
      .asObservable()
      .map(x => x.filter(subscriptionsToEventPredicate(ss)))
      .do(x => log.ws.debug('allEvents', x))
  }
  const persistEvents: RequestPersistEvents = x => {
    log.ws.debug('putEvents', x)
    const newEvents = x.map(payload => ({
      payload: payload.payload,
      name: payload.name,
      semantics: payload.semantics,
      sourceId,
      timestamp: payload.timestamp,
      lamport: Lamport.of(payload.timestamp),
      psn: Psn.of(psn++),
    }))

    events.next(newEvents)
    present.next({ psns: { [sourceId]: psn }, default: 'max' })
    return Observable.of(newEvents)
  }
  return {
    sourceId,
    present: () => present.asObservable().do(() => log.ws.debug('present')),
    highestSeen: () => present.asObservable(),
    persistedEvents,
    allEvents,
    persistEvents,
    connectivityStatus: () => Observable.empty<ConnectivityStatus>(),
  }
}
