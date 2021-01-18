/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { chunksOf } from 'fp-ts/lib/Array'
import { fromNullable } from 'fp-ts/lib/Option'
import { Observable, ReplaySubject, Scheduler, Subject } from 'rxjs'
import log from '../store/loggers'
import { SubscriptionSet, subscriptionsToEventPredicate } from '../subscription'
import { EventKey, Lamport, Psn, SourceId, Timestamp } from '../types'
import { binarySearch, mergeSortedInto } from '../util'
import {
  EventStore,
  RequestAllEvents,
  RequestPersistedEvents,
  RequestPersistEvents,
} from './eventStore'
import {
  AllEventsSortOrders,
  ConnectivityStatus,
  Event,
  Events,
  OffsetMap,
  OffsetMapBuilder,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  PersistedEventsSortOrders,
} from './types'

/**
 * A raw Actyx event to be emitted by the TestEventStore, as if it really arrived from the outside.
 * @public
 */
export type TestEvent = {
  psn: number
  sourceId: string

  timestamp: Timestamp
  lamport: Lamport
  tags: ReadonlyArray<string>

  payload: unknown
}

export type TestEventStore = EventStore & {
  // It is up to the test case to judge which events
  // might realistically appear in the live stream.
  directlyPushEvents: (events: ReadonlyArray<TestEvent>) => void
  storedEvents: () => Event[]
}

const lookup = (offsets: OffsetMap, source: string) => fromNullable(offsets[source])

export type HasPsnAndSource = {
  psn: number
  sourceId: string
}

export const includeEvent = (offsetsBuilder: OffsetMapBuilder, ev: HasPsnAndSource): OffsetMap => {
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

/**
 * HERE BE DRAGONS: This function is just a draft of an optimisation we may do in a Rust-side impl
 * Take an offset map and a sorted array of events -> find an index that fully covers the offsetmap.
 * - Psn of event is smaller than in map -> Too low
 * - Psn of event is higher -> Too high
 * - Event’s source is not in offsets: Too high if offsets default is 'min'
 * - Psn eq: Too low, unless the next event is too high
 */
export const binSearchOffsets = (a: Events, offsets: OffsetMapWithDefault): number => {
  let low = 0
  let high = a.length

  const c = ordOffsetsEvent(offsets, a)

  while (low < high) {
    const mid = (low + high) >>> 1
    if (c(mid) < 0) low = mid + 1
    else high = mid
  }
  return low
}

// For use within `binSearchOffsets` -- very specialized comparison.
const ordOffsetsEvent = (offsets: OffsetMapWithDefault, events: Events) => (i: number): number => {
  const ev = events[i]
  const source = ev.sourceId

  const offset = lookup(offsets.psns, source)

  return offset.fold(
    // Unknown source: Too high.
    1,
    o => {
      const d = ev.psn - o
      if (d !== 0 || i + 1 === events.length) {
        return d
      }

      // If d=0, delegate to the next higher index
      return ordOffsetsEvent(offsets, events)(i + 1)
    },
  )
}

const filterSortedEvents = (
  events: Events,
  from: OffsetMapWithDefault,
  to: OffsetMapWithDefault,
  subs: SubscriptionSet,
  min?: EventKey,
): Event[] => {
  // If min is given, should be among the existing events
  const sliceStart = min ? binarySearch(events, min, EventKey.ord.compare) + 1 : 0

  return events
    .slice(sliceStart)
    .filter(isBetweenPsnLimits(from, to))
    .filter(subscriptionsToEventPredicate(subs))
}

const filterUnsortedEvents = (
  from: OffsetMapWithDefault,
  to: OffsetMapWithDefault,
  subs: SubscriptionSet,
  min?: EventKey,
) => (events: Events): Event[] => {
  return events
    .filter(ev => !min || EventKey.ord.compare(ev, min) > 0)
    .filter(isBetweenPsnLimits(from, to))
    .filter(subscriptionsToEventPredicate(subs))
}

const persistence = () => {
  let persisted: Event[] = []

  const persist = (evsUnsorted: Events) => {
    const evs = [...evsUnsorted].sort(EventKey.ord.compare)
    if (persisted.length === 0) {
      persisted = evs
      return
    }

    if (evs.length === 0) {
      return
    }

    const oldPersisted = [...persisted]

    // Array with lower first element has to go first
    if (EventKey.ord.compare(oldPersisted[0], evs[0]) > 0) {
      persisted = oldPersisted.concat(evs)
      mergeSortedInto(oldPersisted, evs, persisted, EventKey.ord.compare)
    } else {
      persisted = evs.concat(oldPersisted)
      mergeSortedInto(evs, oldPersisted, persisted, EventKey.ord.compare)
    }
  }

  const allPersisted = () => {
    return persisted
  }

  // Get persisted events as a mutable slice with best-effort pre-filtering
  const getPersistedPreFiltered = (
    from: OffsetMapWithDefault,
    to: OffsetMapWithDefault,
  ): Event[] => {
    const events = allPersisted()

    if (OffsetMap.isEmpty(from.psns)) {
      return [...events]
    }

    if (from.default === to.default) {
      return [...events]
    } else {
      // Here be dragons...
      // We actually want to use this when picking up from a snapshot, but that only works when
      // we can guarantee the snapshot is still valid. In case we are hydrating from scratch, we cannot guarantee that!
      // So currently it’s only used for the "live" stream to basically detect that no persisted event needs to be delivered
      // (because in tests the live stream will relibably always start from present and the persisted events will exactly cover the present.)
      const start = binSearchOffsets(events, from)
      return events.slice(start)
    }
  }

  return {
    persist,
    getPersistedPreFiltered,
    allPersisted,
  }
}

export const testEventStore: (sourceId?: SourceId, eventChunkSize?: number) => TestEventStore = (
  sourceId = SourceId.of('TEST'),
  eventChunkSize = 4,
) => {
  const { persist, getPersistedPreFiltered, allPersisted } = persistence()

  const present = new ReplaySubject<OffsetMap>(1)
  const live = new Subject<Events>()

  const persistedEvents: RequestPersistedEvents = (from, to, subs, sortOrder, min) => {
    const events = getPersistedPreFiltered(from, to)

    const filtered = filterSortedEvents(events, from, to, subs, min)

    const ret =
      sortOrder === PersistedEventsSortOrders.ReverseEventKey ? filtered.reverse() : filtered

    return Observable.from(chunksOf(ret, eventChunkSize)).defaultIfEmpty([])
  }

  const liveStream: RequestAllEvents = (from, to, subs, sortOrder, min) => {
    if (sortOrder !== AllEventsSortOrders.Unsorted) {
      throw new Error('The test event store only supports Unsorted ordering')
    }

    return (
      live
        .asObservable()
        .map(filterUnsortedEvents(from, to, subs, min))
        // Delivering live events may trigger new events (via onStateChange) and again new events,
        // until we exhaust the call stack. The prod store shouldn’t have that problem due to obvious reasons.
        .observeOn(Scheduler.queue)
    )
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

  const directlyPushEvents = (newEvents: ReadonlyArray<TestEvent>) => {
    let b = { ...offsets }
    for (const ev of newEvents) {
      b = includeEvent(b, ev)
    }
    offsets = b

    if (newEvents.length > 0) {
      lamport = Lamport.of(Math.max(lamport, ...newEvents.map(x => x.lamport)) + 1)
    }

    const newEventsCompat: Events = newEvents.map(ev => ({
      ...ev,
      semantics: '_t_',
      name: '_t_',
    }))

    persist(newEventsCompat)
    present.next(offsets)
    live.next(newEventsCompat)
  }

  const toIo = (o: OffsetMap): OffsetMapWithDefault => ({ psns: o, default: 'max' })

  return {
    sourceId,
    present: () =>
      present
        .asObservable()
        .map(toIo)
        .do(() => log.ws.debug('present')),
    highestSeen: () => present.asObservable().map(toIo),
    persistedEvents,
    allEvents,
    persistEvents,
    directlyPushEvents,
    storedEvents: allPersisted,
    connectivityStatus: () => Observable.empty<ConnectivityStatus>(),
  }
}
