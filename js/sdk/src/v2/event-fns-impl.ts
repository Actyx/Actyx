/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { chunksOf } from 'fp-ts/lib/Array'
import { contramap, getTupleOrd, gt, lt, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Observable } from 'rxjs'
import {
  AutoCappedQuery,
  EarliestQuery,
  EventFns,
  EventOrder,
  EventSubscription,
  LatestQuery,
  MonotonicSubscription,
  RangeQuery,
} from '../event-fns'
import { SnapshotStore } from '../snapshotStore'
import {
  ActyxEvent,
  allEvents,
  CancelSubscription,
  EventChunk,
  EventsOrTimetravel,
  Metadata,
  MsgType,
  OffsetMap,
  pendingEmission,
  StreamId,
  TaggedEvent,
  Timestamp,
  toMetadata,
  Where,
} from '../types'
import { EventStore } from './eventStore'
import { eventsMonotonic, EventsOrTimetravel as EventsOrTtInternal } from './subscribe_monotonic'
import { AllEventsSortOrders, Event, Events, PersistedEventsSortOrders } from './types'

const ordByTimestamp = contramap(
  (e: ActyxEvent): [number, string] => [e.meta.timestampMicros, e.meta.eventId],
  getTupleOrd(ordNumber, ordString),
)
const ordByKey = contramap((e: ActyxEvent) => e.meta.eventId, ordString)

export const EventFnsFromEventStoreV2 = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
): EventFns => {
  const mkMeta = toMetadata(eventStore.nodeId)

  const wrap = <E>(e: Event): ActyxEvent<E> => ({
    payload: e.payload as E,
    meta: mkMeta(e),
  })

  const bookKeepingOnChunk = (
    initialLowerBound: OffsetMap,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): ((events: Events) => Promise<void>) => {
    const doChunk = chunksOf<Event>(chunkSize)

    let curLowerBound = { ...initialLowerBound }

    const onChunk0 = async (events: Events) => {
      const upperBound = { ...curLowerBound }

      for (const ev of events) {
        upperBound[ev.stream] = ev.offset
      }

      const chunk = {
        events: events.map(wrap),

        // Better pass a copy of our offsets to the client
        upperBound: { ...upperBound },
        lowerBound: { ...curLowerBound },
      }

      curLowerBound = upperBound

      // Promise.resolve converts to a Promise if it's not yet a Promise.
      await onChunk(chunk)
    }

    const onChunk1 = async (events: Events): Promise<void> => {
      for (const chunk of doChunk(events)) {
        await onChunk0(chunk)
      }
    }

    return onChunk1
  }

  const reverseBookKeepingOnChunk = (
    initialUpperBound: OffsetMap,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): ((events: Events) => Promise<void>) => {
    const doChunk = chunksOf<Event>(chunkSize)

    let curUpperBound = { ...initialUpperBound }

    const onChunk0 = async (events: Events) => {
      const lowerBound = { ...curUpperBound }

      const sourcesInChunk = new Set<StreamId>()

      for (const ev of events) {
        lowerBound[ev.stream] = ev.offset
        sourcesInChunk.add(ev.stream)
      }

      for (const src of sourcesInChunk) {
        // lowerbound is *exclusive* meaning we must subtract 1...
        const bound = lowerBound[src]
        if (bound === 0) {
          delete lowerBound[src]
        } else {
          lowerBound[src] = bound - 1
        }
      }

      const chunk = {
        events: events.map(wrap),

        // Better pass a copy of our offsets to the client
        upperBound: { ...curUpperBound },
        lowerBound: { ...lowerBound },
      }

      curUpperBound = lowerBound

      // Promise.resolve converts to a Promise if it's not yet a Promise.
      await Promise.resolve(onChunk(chunk))
    }

    const onChunk1 = async (events: Events) => {
      for (const chunk of doChunk(events)) {
        await onChunk0(chunk)
      }
    }

    return onChunk1
  }

  const currentOffsets = () => eventStore.offsets().then(x => x.present)

  const convertOrder = (ord?: 'Asc' | 'Desc') => {
    return ord === 'Desc'
      ? PersistedEventsSortOrders.Descending
      : PersistedEventsSortOrders.Ascending
  }

  const queryKnownRange = (rangeQuery: RangeQuery) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    return eventStore
      .persistedEvents(
        { default: 'min', psns: lowerBound || {} },
        { default: 'min', psns: upperBound },
        query || allEvents,
        convertOrder(order),
      )
      .concatMap(x => x.map(wrap))
      .toArray()
      .toPromise()
  }

  const queryKnownRangeChunked = (
    rangeQuery: RangeQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => void,
  ) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    const ord = convertOrder(order)

    const lb = lowerBound || {}

    const cb =
      ord === PersistedEventsSortOrders.Ascending
        ? bookKeepingOnChunk(lb, chunkSize, onChunk)
        : reverseBookKeepingOnChunk(upperBound, chunkSize, onChunk)

    return (
      eventStore
        .persistedEvents(
          { default: 'min', psns: lb },
          { default: 'min', psns: upperBound },
          query || allEvents,
          ord,
        )
        // The only way to avoid parallel invocations is to use mergeScan with final arg=1
        .mergeScan((_a: void, chunk: Events) => Observable.from(cb(chunk)), void 0, 1)
        .toPromise()
    )
  }

  const queryAllKnown = async (query: AutoCappedQuery): Promise<EventChunk> => {
    const present = await currentOffsets()

    const rangeQuery = {
      ...query,
      upperBound: present,
    }

    const events = await queryKnownRange(rangeQuery)

    return { events, lowerBound: query.lowerBound || {}, upperBound: present }
  }

  const queryAllKnownChunked = async (
    query: AutoCappedQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): Promise<OffsetMap> => {
    const present = await currentOffsets()

    const rangeQuery = {
      ...query,
      upperBound: present,
    }

    return queryKnownRangeChunked(rangeQuery, chunkSize, onChunk).then(() => present)
  }

  const subscribe = (
    openQuery: EventSubscription,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): CancelSubscription => {
    const { lowerBound, query, maxChunkSize } = openQuery
    const lb = lowerBound || {}

    const cb = bookKeepingOnChunk(lb, maxChunkSize || 5000, onChunk)

    const x = eventStore
      .allEvents(
        { psns: lb, default: 'min' },
        { psns: {}, default: 'max' },
        query || allEvents,
        AllEventsSortOrders.Unsorted,
      )
      // The only way to avoid parallel invocations is to use mergeScan with final arg=1
      .mergeScan((_a: void, chunk: Events) => Observable.from(cb(chunk)), void 0, 1)
      .subscribe()

    return () => x.unsubscribe()
  }

  const convertMsg = <E>(m: EventsOrTtInternal): EventsOrTimetravel<E> => {
    switch (m.type) {
      case MsgType.state:
        return m
      case MsgType.events:
        return {
          type: MsgType.events,
          events: m.events.map(wrap) as ActyxEvent<E>[],
          caughtUp: m.caughtUp,
        }
      case MsgType.timetravel:
        return {
          type: MsgType.timetravel,
          trigger: wrap<E>(m.trigger),
          high: wrap<E>(m.high),
        }
      default:
        throw new Error('Unknown msg type in: ' + JSON.stringify(m))
    }
  }

  const subMono = eventsMonotonic(eventStore, snapshotStore)
  const subscribeMonotonic = <E>(
    query: MonotonicSubscription<E>,
    cb: (data: EventsOrTimetravel<E>) => Promise<void> | void,
  ): CancelSubscription => {
    const x = subMono(query.sessionId, query.query, query.attemptStartFrom)
      .map(x => convertMsg<E>(x))
      // The only way to avoid parallel invocations is to use mergeScan with final arg=1
      .mergeScan(
        (_a: void, m: EventsOrTimetravel<E>) => Observable.from(Promise.resolve(cb(m))),
        void 0,
        1,
      )
      .subscribe()

    return () => x.unsubscribe()
  }

  // Find first currently known event according to given sorting
  const findFirstKnown = async <E>(
    query: Where<E>,
    order: PersistedEventsSortOrders,
  ): Promise<[ActyxEvent<E> | undefined, OffsetMap]> => {
    const cur = await currentOffsets()

    const firstEvent = await eventStore
      .persistedEvents({ psns: {}, default: 'min' }, { psns: cur, default: 'min' }, query, order)
      .concatMap(x => x)
      .first()
      .toPromise()

    return [wrap(firstEvent), cur]
  }

  // Find first currently known event according to an arbitrary decision logic
  const reduceUpToPresent = async <R, E = unknown>(
    query: Where<E>,
    reduce: (acc: R, e1: ActyxEvent<E>) => R,
    initial: R,
  ): Promise<[R, OffsetMap]> => {
    const cur = await currentOffsets()

    const reducedValue = await eventStore
      .persistedEvents(
        { psns: {}, default: 'min' },
        { psns: cur, default: 'min' },
        query,
        // Doesn't matter, we have to go through all known events anyways
        PersistedEventsSortOrders.Ascending,
      )
      .concatMap(x => x.map(e => wrap<E>(e)))
      .reduce(reduce, initial)
      .toPromise()

    return [reducedValue, cur]
  }

  const callbackWhenReplaced = <E>(
    query: Where<E>,
    startingOffsets: OffsetMap,
    initial: ActyxEvent<E> | undefined,
    onEvent: (event: E, metadata: Metadata) => void,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
  ): CancelSubscription => {
    let cur = initial

    if (cur) {
      onEvent(cur.payload as E, cur.meta)
    }

    const cb = async (boxedChunk: EventChunk) => {
      const untypedChunk = boxedChunk.events

      if (untypedChunk.length === 0) {
        return
      }
      const chunk = untypedChunk as ActyxEvent<E>[]

      let replaced = false

      // Chunk is NOT sorted internally in live-mode. Any event may replace cur.
      for (const event of chunk) {
        if (!cur || shouldReplace(event, cur)) {
          cur = event
          replaced = true
        }
      }

      // Replaced=true implies cur!=null, but the compiler doesn't know.
      if (replaced && cur) {
        onEvent(cur.payload as E, cur.meta)
      }
    }

    return subscribe({ query, lowerBound: startingOffsets }, cb)
  }

  const observeBestMatch = <E>(
    query: Where<E>,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
    onReplaced: (event: E, metadata: Metadata) => void,
  ): CancelSubscription => {
    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    reduceUpToPresent<ActyxEvent<E> | undefined, E>(
      query,
      (e0, e1) => (e0 && shouldReplace(e0, e1) ? e0 : e1),
      undefined,
    ).then(([initial, offsets]) => {
      if (cancelled) {
        return
      }

      cancelSubscription = callbackWhenReplaced(query, offsets, initial, onReplaced, shouldReplace)
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const observeEarliest = <E>(
    tq: EarliestQuery<E>,
    onEvent: (event: E, metadata: Metadata) => void,
  ): CancelSubscription => {
    const { query, eventOrder } = tq

    if (eventOrder === EventOrder.Timestamp) {
      return observeBestMatch(query, lt(ordByTimestamp), onEvent)
    }

    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    /** If lamport order is desired, we can use store-support to speed up the query. */
    findFirstKnown(query, PersistedEventsSortOrders.Ascending).then(([earliest, offsets]) => {
      if (cancelled) {
        return
      }

      cancelSubscription = callbackWhenReplaced(query, offsets, earliest, onEvent, lt(ordByKey))
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const observeLatest = <E>(
    tq: LatestQuery<E>,
    onEvent: (event: E, metadata: Metadata) => void,
  ): CancelSubscription => {
    const { query, eventOrder } = tq

    if (eventOrder === EventOrder.Timestamp) {
      return observeBestMatch(query, gt(ordByTimestamp), onEvent)
    }

    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    /** If lamport order is desired, we can use store-support to speed up the query. */
    findFirstKnown(query, PersistedEventsSortOrders.Descending).then(([latest, offsets]) => {
      if (cancelled) {
        return
      }

      cancelSubscription = callbackWhenReplaced(query, offsets, latest, onEvent, gt(ordByKey))
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const observeUnorderedReduce = <R, E>(
    query: Where<E>,
    reduce: (acc: R, event: E, metadata: Metadata) => R,
    initialVal: R,
    onUpdate: (result: R) => void,
  ): CancelSubscription => {
    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    const reduceDirect = (r: R, evt: ActyxEvent) => reduce(r, evt.payload as E, evt.meta)

    reduceUpToPresent<R>(query, reduceDirect, initialVal).then(([initial, offsets]) => {
      if (cancelled) {
        return
      }

      let cur = initial
      onUpdate(cur)

      const cb = async (chunk: EventChunk) => {
        if (chunk.events.length === 0) {
          return
        }

        cur = chunk.events.reduce(reduceDirect, cur)
        onUpdate(cur)
      }

      cancelSubscription = subscribe({ query, lowerBound: offsets }, cb)
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const emit = (taggedEvents: ReadonlyArray<TaggedEvent>) => {
    const events = taggedEvents.map(({ tags, event }) => {
      const timestamp = Timestamp.now()

      return {
        tags,
        timestamp, // FIXME
        payload: event,
      }
    })

    const allPersisted = eventStore
      .persistEvents(events)
      .toArray()
      .map(x => x.flat().map(mkMeta))
      .shareReplay(1)

    return pendingEmission(allPersisted)
  }

  return {
    nodeId: eventStore.nodeId,
    currentOffsets,
    queryKnownRange,
    queryKnownRangeChunked,
    queryAllKnown,
    queryAllKnownChunked,
    subscribe,
    subscribeMonotonic,
    observeEarliest,
    observeLatest,
    observeBestMatch,
    observeUnorderedReduce,
    emit,
  }
}
