/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import {
  fromNullable,
  exists as existsO,
  map as mapO,
  getOrElse as getOrElseO,
  fold as foldO,
} from 'fp-ts/lib/Option'
import {
  ReplaySubject,
  Subject,
  from as fromRx,
  queueScheduler,
  concat,
  defer,
  of,
  lastValueFrom,
  EMPTY,
} from '../../node_modules/rxjs'
import { mergeMap, observeOn, first, map } from '../../node_modules/rxjs/operators'
import {
  AppId,
  EventKey,
  EventsSortOrder,
  Lamport,
  NodeId,
  Offset,
  OffsetMap,
  OffsetMapBuilder,
  TimeInjector,
  Timestamp,
  toEventPredicate,
  Where,
} from '../types'
import { binarySearch, mergeSortedInto } from '../util'
import { DoPersistEvents, DoQuery, DoSubscribe, EventStore } from './eventStore'
import { Event, Events } from './types'

/**
 * A raw Actyx event to be emitted by the TestEventStore, as if it really arrived from the outside.
 * @public
 */
export type TestEvent = {
  offset: number
  stream: string

  timestamp: Timestamp
  lamport: Lamport
  tags: string[]

  payload: unknown
}

export type TestEventStore = EventStore & {
  // It is up to the test case to judge which events
  // might realistically appear in the live stream.
  directlyPushEvents: (events: TestEvent[]) => void
  storedEvents: () => Event[]

  // End all streams. The real store is not expected to do this.
  close: () => void
}

const lookup = (offsets: OffsetMap, source: string) => fromNullable(offsets[source])

export type HasPsnAndSource = {
  offset: number
  stream: string
}

export const includeEvent = (offsetsBuilder: OffsetMapBuilder, ev: HasPsnAndSource): OffsetMap => {
  const { offset, stream } = ev
  const current = lookup(offsetsBuilder, stream)
  if (!existsO((c: number) => c >= offset)(current)) {
    offsetsBuilder[stream] = offset
  }
  return offsetsBuilder
}

const isBetweenPsnLimits =
  (from: OffsetMap, to: OffsetMap, onboardNewSources: boolean) => (e: Event) => {
    const source: string = e.stream

    const lower = lookup(from, source)
    const upper = lookup(to, source)

    const passLower = getOrElseO(() => true)(mapO((lw: number) => e.offset > lw)(lower))
    const passUpper = getOrElseO(() => onboardNewSources)(
      mapO((up: number) => e.offset <= up)(upper),
    )

    return passLower && passUpper
  }

/**
 * HERE BE DRAGONS: This function is just a draft of an optimisation we may do in a Rust-side impl
 * Take an offset map and a sorted array of events -> find an index that fully covers the offsetmap.
 * - Offset of event is smaller than in map -> Too low
 * - Offset of event is higher -> Too high
 * - Event’s source is not in offsets: Too high if offsets default is 'min'
 * - Psn eq: Too low, unless the next event is too high
 */
export const binSearchOffsets = (a: Events, offsets: OffsetMap): number => {
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
const ordOffsetsEvent =
  (offsets: OffsetMap, events: Events) =>
  (i: number): number => {
    const ev = events[i]
    const source = ev.stream

    const offset = lookup(offsets, source)

    return foldO(
      // Unknown source: Too high.
      () => 1,
      (o: number) => {
        const d = ev.offset - o
        if (d !== 0 || i + 1 === events.length) {
          return d
        }

        // If d=0, delegate to the next higher index
        return ordOffsetsEvent(offsets, events)(i + 1)
      },
    )(offset)
  }

const filterSortedEvents = (
  events: Events,
  from: OffsetMap,
  to: OffsetMap,
  subs: Where<unknown>,
  min?: EventKey,
): Event[] => {
  // If min is given, should be among the existing events
  const sliceStart = min ? binarySearch(events, min, EventKey.ord.compare) + 1 : 0

  return events
    .slice(sliceStart)
    .filter(isBetweenPsnLimits(from, to, false))
    .filter(toEventPredicate(subs))
}

const filterUnsortedEvents =
  (from: OffsetMap, to: OffsetMap, subs: Where<unknown>, min?: EventKey) =>
  (events: Events): Event[] => {
    return events
      .filter((ev) => !min || EventKey.ord.compare(ev, min) > 0)
      .filter(isBetweenPsnLimits(from, to, true))
      .filter(toEventPredicate(subs))
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
  const getPersistedPreFiltered = (from: OffsetMap, _to: OffsetMap): Event[] => {
    const events = allPersisted()

    if (OffsetMap.isEmpty(from)) {
      return [...events]
    }

    return [...events]

    // Here be dragons...
    // We actually want to use this when picking up from a snapshot, but that only works when
    // we can guarantee the snapshot is still valid. In case we are hydrating from scratch, we cannot guarantee that!
    // So currently it’s only used for the "live" stream to basically detect that no persisted event needs to be delivered
    // (because in tests the live stream will relibably always start from present and the persisted events will exactly cover the present.)

    // const start = binSearchOffsets(events, from)
    // return events.slice(start)
  }

  return {
    persist,
    getPersistedPreFiltered,
    allPersisted,
  }
}

export const testEventStore = (nodeId: NodeId = NodeId.of('TEST'), timeInjector?: TimeInjector) => {
  const { persist, getPersistedPreFiltered, allPersisted } = persistence()
  const time = timeInjector || (() => Timestamp.now())

  const present = new ReplaySubject<OffsetMap>(1)
  const live = new Subject<Events>()

  const query: DoQuery = (from, to, subs, sortOrder) => {
    const events = getPersistedPreFiltered(from, to)

    if (typeof subs === 'string') {
      throw new Error('direct AQL not yet supported by testEventStore')
    }

    const filtered = filterSortedEvents(events, from, to, subs)

    const ret = sortOrder === EventsSortOrder.Descending ? filtered.reverse() : filtered

    return fromRx(ret)
  }

  const liveStream: DoSubscribe = (from, subs) => {
    if (typeof subs === 'string') {
      throw new Error('direct AQL not yet supported by testEventStore')
    }

    return live.asObservable().pipe(
      mergeMap((x) => fromRx(filterUnsortedEvents(from, {}, subs)(x))),
      // Delivering live events may trigger new events (via onStateChange) and again new events,
      // until we exhaust the call stack. The prod store shouldn’t have that problem due to obvious reasons.
      observeOn(queueScheduler),
    )
  }

  let curOffsets = {}
  present.next(curOffsets)

  const subscribe: DoSubscribe = (fromPsn, subs) => {
    const k = () => {
      return concat(
        query(fromPsn, curOffsets, subs, EventsSortOrder.StreamAscending),
        liveStream(fromPsn, subs),
      )
    }

    return defer(k)
  }

  let psn = 0

  let lamport = Lamport.of(99999)

  const streamId = NodeId.streamNo(nodeId, 0)

  const persistEvents: DoPersistEvents = (x) => {
    const newEvents = x.map((unstoredEvent) => {
      lamport = Lamport.of(lamport + 1)
      return {
        ...unstoredEvent,
        appId: AppId.of('test'),
        stream: streamId,
        lamport,
        timestamp: time(unstoredEvent.tags, unstoredEvent.payload),
        offset: Offset.of(psn++),
      }
    })

    directlyPushEvents(newEvents)
    return of(newEvents)
  }

  const directlyPushEvents = (newEvents: TestEvent[]) => {
    let b = { ...curOffsets }
    for (const ev of newEvents) {
      b = includeEvent(b, ev)
    }
    curOffsets = b

    if (newEvents.length > 0) {
      lamport = Lamport.of(Math.max(lamport, ...newEvents.map((x) => x.lamport)) + 1)
    }

    const newEventsCompat: Events = newEvents.map((ev) => ({
      ...ev,
      semantics: '_t_',
      name: '_t_',
      appId: AppId.of('test'),
    }))

    persist(newEventsCompat)
    present.next(curOffsets)
    live.next(newEventsCompat)
  }

  const getPresent = () =>
    lastValueFrom(
      present.asObservable().pipe(
        first(),
        map((present) => ({ present, toReplicate: {} })),
      ),
    )

  return {
    nodeId,
    offsets: getPresent,
    query,
    queryUnchecked: () => {
      throw new Error('not implemented for test event store')
    },
    subscribe,
    subscribeUnchecked: () => {
      throw new Error('not implemented for test event store')
    },
    subscribeMonotonic: () => {
      throw new Error('not implemented for test event store')
    },
    persistEvents,
    directlyPushEvents,
    storedEvents: allPersisted,
    connectivityStatus: () => EMPTY,
    close: () => live.complete(),
  }
}
