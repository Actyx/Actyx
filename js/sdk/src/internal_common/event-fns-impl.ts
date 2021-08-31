/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { contramap, getTupleOrd, gt, lt, ordNumber, ordString } from 'fp-ts/lib/Ord'
import { Observable } from '../../node_modules/rxjs'
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
  AqlResponse,
  CancelSubscription,
  EventChunk,
  EventKey,
  EventsOrTimetravel,
  EventsSortOrder,
  Metadata,
  MsgType,
  NodeId,
  OffsetMap,
  pendingEmission,
  StreamId,
  TaggedEvent,
  toMetadata,
  Where,
} from '../types'
import { EventStore } from './eventStore'
import { eventsMonotonic, EventsOrTimetravel as EventsOrTtInternal } from './subscribe_monotonic'
import { Event, Events } from './types'

const ordByTimestamp = contramap(
  (e: ActyxEvent): [number, string] => [e.meta.timestampMicros, e.meta.eventId],
  getTupleOrd(ordNumber, ordString),
)
const ordByKey = contramap((e: ActyxEvent) => e.meta.eventId, ordString)

export const EventFnsFromEventStoreV2 = (
  nodeId: NodeId,
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
): EventFns => {
  const mkMeta = toMetadata(nodeId)

  const wrap = <E>(e: Event): ActyxEvent<E> => ({
    payload: e.payload as E,
    meta: mkMeta(e),
  })

  const bookKeepingOnChunk = (
    initialLowerBound: OffsetMap,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): ((preChunkedEvents: Events) => Promise<void>) => {
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
      await Promise.resolve(onChunk(chunk))
    }

    return onChunk0
  }

  const reverseBookKeepingOnChunk = (
    initialUpperBound: OffsetMap,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): ((preChunkedEvents: Events) => Promise<void>) => {
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

    return onChunk0
  }

  const present = () => eventStore.offsets().then(x => x.present)

  const offsets = () => eventStore.offsets()

  const queryKnownRange = (rangeQuery: RangeQuery) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    return eventStore
      .query(lowerBound || {}, upperBound, query || allEvents, order || EventsSortOrder.Ascending)
      .map(wrap)
      .toArray()
      .toPromise()
  }

  const queryKnownRangeChunked = (
    rangeQuery: RangeQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => void,
    onComplete?: () => void,
  ) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    const lb = lowerBound || {}

    const cb =
      order === EventsSortOrder.Ascending
        ? bookKeepingOnChunk(lb, onChunk)
        : reverseBookKeepingOnChunk(upperBound, onChunk)

    let cancelled = false

    const s = eventStore
      .query(lb, upperBound, query || allEvents, order || EventsSortOrder.Ascending)
      .bufferCount(chunkSize)
      // The only way to avoid parallel invocations is to use mergeScan with final arg=1
      .mergeScan(
        (_a: void, chunk: Events) => {
          return cancelled ? Observable.empty<void>() : Observable.from(cb(chunk))
        },
        void 0,
        1,
      )
      .subscribe()

    if (onComplete instanceof Function) {
      s.add(onComplete)
    }

    return () => {
      cancelled = true
      s.unsubscribe()
    }
  }

  const queryAllKnown = async (query: AutoCappedQuery): Promise<EventChunk> => {
    const curPresent = await present()

    const rangeQuery = {
      ...query,
      upperBound: curPresent,
    }

    const events = await queryKnownRange(rangeQuery)

    return { events, lowerBound: query.lowerBound || {}, upperBound: curPresent }
  }

  const queryAllKnownChunked = (
    query: AutoCappedQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
    onComplete?: () => void,
  ) => {
    let canceled = false
    let cancelUpstream = () => {
      onComplete && onComplete()
      // Function is bound again when the real query starts
    }

    present().then(present => {
      if (canceled) {
        return
      }

      const rangeQuery = {
        ...query,
        upperBound: present,
      }

      cancelUpstream = queryKnownRangeChunked(rangeQuery, chunkSize, onChunk, onComplete)
    })

    return () => {
      canceled = true
      cancelUpstream()
    }
  }
  const subscribe = (
    openQuery: EventSubscription,
    onEvent: (e: ActyxEvent) => Promise<void> | void,
  ): CancelSubscription => {
    const { lowerBound, query } = openQuery
    const lb = lowerBound || {}

    const rxSub = eventStore
      .subscribe(lb, query || allEvents)
      .map(wrap)
      .mergeScan(
        (_a: void, e: ActyxEvent) => Observable.from(Promise.resolve(onEvent(e))),
        void 0,
        1,
      )
      .subscribe()

    return () => rxSub.unsubscribe()
  }

  const subscribeChunked = (
    openQuery: EventSubscription,
    cfg: { maxChunkSize?: number; maxChunkTimeMs?: number },
    onChunk: (chunk: EventChunk) => Promise<void> | void,
  ): CancelSubscription => {
    const { lowerBound, query } = openQuery
    const lb = lowerBound || {}

    const cb = bookKeepingOnChunk(lb, onChunk)

    const bufTime = cfg.maxChunkTimeMs || 5
    const bufSize = cfg.maxChunkSize || 1000
    const s = eventStore.subscribe(lb, query || allEvents)

    const buffered = s
      // 2nd arg to bufferTime is not marked as optional, but it IS optional
      /* eslint-disable-next-line @typescript-eslint/no-non-null-assertion */
      .bufferTime(bufTime, null!, bufSize)
      .filter(x => x.length > 0)
      .map(buf => buf.sort(EventKey.ord.compare))

    // The only way to avoid parallel invocations is to use mergeScan with final arg=1
    const rxSub = buffered
      .mergeScan((_a: void, chunk: Events) => Observable.from(cb(chunk)), void 0, 1)
      .subscribe()

    return () => rxSub.unsubscribe()
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
    order: EventsSortOrder,
  ): Promise<[ActyxEvent<E> | undefined, OffsetMap]> => {
    const cur = await present()

    const firstEvent = await eventStore
      .query({}, cur, query, order)
      .defaultIfEmpty(null)
      .first()
      .toPromise()

    return [firstEvent ? wrap(firstEvent) : undefined, cur]
  }

  // Find first currently known event according to an arbitrary decision logic
  const reduceUpToPresent = async <R, E = unknown>(
    query: Where<E>,
    reduce: (acc: R, e1: ActyxEvent<E>) => R,
    initial: R,
  ): Promise<[R, OffsetMap]> => {
    const cur = await present()

    const reducedValue = await eventStore
      .query(
        {},
        cur,
        query,
        // Doesn't matter, we have to go through all known events anyways
        EventsSortOrder.Ascending,
      )
      .map(e => wrap<E>(e))
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
      // (Actually, we are now internally sorting. Maybe this can be improved.)
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

    return subscribeChunked({ query, lowerBound: startingOffsets }, {}, cb)
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
      (e0, e1) => (!e0 || shouldReplace(e1, e0) ? e1 : e0),
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
    findFirstKnown(query, EventsSortOrder.Ascending).then(([earliest, offsets]) => {
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
    findFirstKnown(query, EventsSortOrder.Descending).then(([latest, offsets]) => {
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

      cancelSubscription = subscribeChunked({ query, lowerBound: offsets }, {}, cb)
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const emit = (taggedEvents: ReadonlyArray<TaggedEvent>) => {
    const events = taggedEvents.map(({ tags, event }) => {
      return {
        tags,
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

  // TS doesn’t understand how we are implementing this overload.
  // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
  // @ts-ignore
  const publish: EventFns['publish'] = (taggedEvents: ReadonlyArray<TaggedEvent> | TaggedEvent) => {
    if (Array.isArray(taggedEvents)) {
      return emit(taggedEvents).toPromise()
    } else {
      return emit([taggedEvents as TaggedEvent])
        .toPromise()
        .then(x => x[0])
    }
  }

  // FIXME properly type EventStore. (This runs without error because in production mode the ws event store does not use io-ts.)
  const wrapAql = (e: { type: string }): AqlResponse => {
    const actualType = e.type

    if (actualType === 'offsets' || actualType === 'diagnostic') {
      return e as AqlResponse
    }

    const w = wrap((e as unknown) as Event)

    return {
      ...w,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      type: actualType as any,
    }
  }

  const queryAql = async (query: string): Promise<AqlResponse[]> => {
    return eventStore
      .queryUnchecked(query, EventsSortOrder.Ascending)
      .map(wrapAql)
      .toArray()
      .toPromise()
  }

  const queryAqlChunked = (
    query: {
      query: string
      chunkSize?: number
      ord?: EventsSortOrder
    },
    onChunk: (chunk: AqlResponse[]) => Promise<void> | void,
  ): CancelSubscription => {
    const buffered = eventStore
      .queryUnchecked(query.query, query.ord || EventsSortOrder.Ascending)
      .map(wrapAql)
      .bufferCount(query.chunkSize || 128)

    // The only way to avoid parallel invocations is to use mergeScan with final arg=1
    const rxSub = buffered
      .mergeScan(
        (_a: void, chunk: AqlResponse[]) => Observable.from(Promise.resolve(onChunk(chunk))),
        void 0,
        1,
      )
      .subscribe()

    return () => rxSub.unsubscribe()
  }

  return {
    present,
    offsets,
    queryKnownRange,
    queryKnownRangeChunked,
    queryAllKnown,
    queryAllKnownChunked,
    queryAql,
    queryAqlChunked,
    subscribe,
    subscribeChunked,
    subscribeMonotonic,
    observeEarliest,
    observeLatest,
    observeBestMatch,
    observeUnorderedReduce,
    emit,
    publish,
  }
}
