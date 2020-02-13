import { chunksOf } from 'fp-ts/lib/Array'
import { fromNullable } from 'fp-ts/lib/Option'
import { Observable, ReplaySubject, Subject, Scheduler } from 'rxjs'
import log from '../store/loggers'
import { SubscriptionSet, subscriptionsToEventPredicate } from '../subscription'
import { EventKey, Lamport, Psn, SourceId } from '../types'
import {
  EventStore,
  RequestAllEvents,
  RequestPersistedEvents,
  RequestPersistEvents,
} from './eventStore'
import {
  AllEventsSortOrders,
  Event,
  Events,
  OffsetMap,
  OffsetMapBuilder,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  PersistedEventsSortOrders,
  ConnectivityStatus,
} from './types'

export type TestEventStore = EventStore & {
  // It is up to the test case to judge which events
  // might realistically appear in the live stream.
  directlyPushEvents: (events: Events) => void
  storedEvents: () => Event[]
}

const lookup = (offsets: OffsetMap, source: string) => fromNullable(offsets[source])

export const includeEvent = (offsetsBuilder: OffsetMapBuilder, ev: Event): OffsetMap => {
  const { psn, sourceId } = ev
  const current = lookup(offsetsBuilder, sourceId)
  if (!current.exists(c => c >= psn)) {
    offsetsBuilder[sourceId] = psn
  }
  return offsetsBuilder
}

const isBetweenPsnLimits = (from: OffsetMapWithDefault, to: OffsetMapWithDefault) => (e: Event) => {
  const source: string = e.sourceId

  const lower = lookup(from.psns, source)
  const upper = lookup(to.psns, source)

  const passLower = lower.map(lw => e.psn > lw).getOrElse(from.default === 'min')
  const passUpper = upper.map(up => e.psn <= up).getOrElse(to.default === 'max')

  return passLower && passUpper
}

const filterEvents = (
  from: OffsetMapWithDefault,
  to: OffsetMapWithDefault,
  subs: SubscriptionSet,
  min?: EventKey,
) => (events: Events): Events => {
  return events
    .filter(e => !min || EventKey.ord.compare(EventKey.fromEnvelope0(e), min) >= 0)
    .filter(isBetweenPsnLimits(from, to))
    .filter(subscriptionsToEventPredicate(subs))
}

const persistence = () => {
  let sorted = true
  const persisted: Event[] = []

  const persist = (evs: Events) => {
    persisted.push(...evs)
    sorted = false
  }
  const getPersisted = () => {
    if (!sorted) {
      persisted.sort(EventKey.ord.compare)
      sorted = true
    }

    return persisted
  }

  return {
    persist,
    getPersisted,
  }
}

export const testEventStore: (sourceId?: SourceId, eventChunkSize?: number) => TestEventStore = (
  sourceId = SourceId.of('TEST'),
  eventChunkSize = 4,
) => {
  const { persist, getPersisted } = persistence()

  const present = new ReplaySubject<OffsetMap>(1)
  const live = new Subject<Events>()

  const persistedEvents: RequestPersistedEvents = (from, to, subs, sortOrder, min) => {
    const events = getPersisted()

    const ret =
      sortOrder === PersistedEventsSortOrders.ReverseEventKey ? [...events].reverse() : events

    return Observable.from(chunksOf(ret, eventChunkSize)).map(filterEvents(from, to, subs, min))
  }

  const liveStream: RequestAllEvents = (from, to, subs, sortOrder, min) => {
    if (sortOrder !== AllEventsSortOrders.Unsorted) {
      throw new Error('The test event store only supports Unsorted ordering')
    }

    return live.asObservable().map(filterEvents(from, to, subs, min))
      // Delivering live events may trigger new events (via onStateChange) and again new events,
      // until we exhaust the call stack. The prod store shouldn’t have that problem due to obvious reasons.
      .observeOn(Scheduler.queue)
  }

  const allEvents: RequestAllEvents = (fromPsn, toPsn, subs, sortOrder, min) => {
    const k = () => {
      return Observable.concat(
        persistedEvents(
          fromPsn,
          toPsn,
          subs,
          (sortOrder as string) as PersistedEventsSortOrder,
          min,
        ),
        liveStream(fromPsn, toPsn, subs, sortOrder, min),
      )
    }

    return Observable.defer(k)
  }

  let psn = 0

  let lamport = Lamport.of(99999)

  let offsets = {}
  present.next(offsets)

  const persistEvents: RequestPersistEvents = x => {
    const newEvents = x.map(unstoredEvent => {
      lamport = Lamport.of(lamport + 1)
      return {
        ...unstoredEvent,
        sourceId,
        lamport,
        psn: Psn.of(psn++),
      }
    })

    directlyPushEvents(newEvents)
    return Observable.of(newEvents)
  }

  const directlyPushEvents = (newEvents: Events) => {
    let b = { ...offsets }
    for (const ev of newEvents) {
      b = includeEvent(b, ev)
    }
    offsets = b

    if (newEvents.length > 0) {
      lamport = Lamport.of(Math.max(newEvents[newEvents.length - 1].lamport + 1, lamport))
    }

    persist(newEvents)
    live.next(newEvents)
    present.next(offsets)
  }

  const toIo = (o: OffsetMap): OffsetMapWithDefault => ({ psns: o, default: 'max' })

  return {
    sourceId,
    present: () =>
      present
        .asObservable()
        .map(toIo)
        .do(() => log.ws.debug('present')),
    highestSeen: () =>
      present
        .asObservable()
        .map(toIo),
    persistedEvents,
    allEvents,
    persistEvents,
    directlyPushEvents,
    storedEvents: () => getPersisted(),
    connectivityStatus: () => Observable.empty<ConnectivityStatus>()
  }
}
