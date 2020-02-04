import { chunksOf } from 'fp-ts/lib/Array'
import { fromNullable } from 'fp-ts/lib/Option'
import { Observable, ReplaySubject, Subject } from 'rxjs'
import log from '../store/loggers'
import { SubscriptionSet, subscriptionsToEventPredicate } from '../subscription'
import { EventKey, Lamport, Psn, SourceId, Timestamp } from '../types'
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

export const testEventStore: (sourceId: SourceId, eventChunkSize?: number) => TestEventStore = (
  sourceId,
  eventChunkSize,
) => {
  const storedSorted: Event[] = []

  const present = new ReplaySubject<OffsetMap>(1)
  const live = new Subject<Events>()

  const persistedEvents: RequestPersistedEvents = (from, to, subs, sortOrder, min) => {
    const ret =
      sortOrder === PersistedEventsSortOrders.ReverseEventKey
        ? [...storedSorted].reverse()
        : storedSorted

    return Observable.from(chunksOf(ret, eventChunkSize || 4)).map(
      filterEvents(from, to, subs, min),
    )
  }

  const liveStream: RequestAllEvents = (from, to, subs, sortOrder, min) => {
    if (sortOrder !== AllEventsSortOrders.Unsorted) {
      throw new Error('The test event store only supports Unsorted ordering')
    }

    return live.asObservable().map(filterEvents(from, to, subs, min))
  }

  const allEvents: RequestAllEvents = (fromPsn, toPsn, subs, sortOrder, min) => {
    return Observable.concat(
      persistedEvents(fromPsn, toPsn, subs, (sortOrder as string) as PersistedEventsSortOrder, min),
      liveStream(fromPsn, toPsn, subs, sortOrder, min),
    )
  }

  let psn = 0

  let lamport = Lamport.of(Timestamp.now())

  let offsets = {}
  present.next(offsets)

  const persistEvents: RequestPersistEvents = x => {
    const newEvents = x.map(unstoredEvent => {
      lamport = Lamport.of(Math.max(Timestamp.now(), lamport + 1))
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

    storedSorted.push(...newEvents)
    storedSorted.sort(EventKey.ord.compare)
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
    persistedEvents,
    allEvents,
    persistEvents,
    directlyPushEvents,
    storedEvents: () => storedSorted,
  }
}
