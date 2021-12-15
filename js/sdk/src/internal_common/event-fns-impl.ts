/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { contramap, gt, lt, tuple } from 'fp-ts/lib/Ord'
import { Ord as StringOrd } from 'fp-ts/string'
import { Ord as NumberOrd } from 'fp-ts/number'
import { SubscribeAqlProps } from '..'
import { lastValueFrom, EMPTY, from, defaultIfEmpty, first } from '../../node_modules/rxjs'
import {
  map,
  filter,
  toArray,
  bufferCount,
  mergeScan,
  bufferTime,
  reduce as rxReduce,
  shareReplay,
} from '../../node_modules/rxjs/operators'
import {
  AqlQuery,
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
  isString,
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
import { noop } from '../util'
import { EventStore } from './eventStore'
import { eventsMonotonic, EventsOrTimetravel as EventsOrTtInternal } from './subscribe_monotonic'
import { Event, Events } from './types'

export const _ordByTimestamp = contramap((e: ActyxEvent): [number, string] => [
  e.meta.timestampMicros,
  e.meta.eventId,
])(tuple(NumberOrd, StringOrd))
export const _ordByKey = contramap((e: ActyxEvent) => e.meta.eventId)(StringOrd)

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

  const present = () => eventStore.offsets().then((x) => x.present)

  const offsets = () => eventStore.offsets()

  const queryKnownRange = (rangeQuery: RangeQuery) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    return lastValueFrom(
      eventStore
        .query(lowerBound || {}, upperBound, query || allEvents, order || EventsSortOrder.Ascending)
        .pipe(map(wrap), toArray()),
    )
  }

  const queryKnownRangeChunked = (
    rangeQuery: RangeQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => void,
    onComplete?: (err?: unknown) => void,
  ) => {
    const { lowerBound, upperBound, query, order } = rangeQuery

    const lb = lowerBound || {}

    const cb =
      order === EventsSortOrder.Ascending
        ? bookKeepingOnChunk(lb, onChunk)
        : reverseBookKeepingOnChunk(upperBound, onChunk)

    let cancelled = false

    const onCompleteOrErr = onComplete ? onComplete : noop

    const s = eventStore
      .query(lb, upperBound, query || allEvents, order || EventsSortOrder.Ascending)
      .pipe(
        bufferCount(chunkSize),
        mergeScan(
          (_a: void, chunk: Events) => {
            return cancelled ? EMPTY : from(cb(chunk))
          },
          void 0,
          1,
        ),
      )
      // The only way to avoid parallel invocations is to use mergeScan with final arg=1
      .subscribe({ complete: onCompleteOrErr, error: onCompleteOrErr })

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
    onComplete?: (err?: unknown) => void,
  ) => {
    let canceled = false
    let cancelUpstream = () => {
      onComplete && onComplete()
      // Function is bound again when the real query starts
    }

    present().then((present) => {
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
    onError?: (err: unknown) => void,
  ): CancelSubscription => {
    const { lowerBound, query } = openQuery
    const lb = lowerBound || {}

    const rxSub = eventStore
      .subscribe(lb, query || allEvents)
      .pipe(
        map(wrap),
        mergeScan((_a: void, e: ActyxEvent) => from(Promise.resolve(onEvent(e))), void 0, 1),
      )
      .subscribe({ error: onError || noop })

    return () => rxSub.unsubscribe()
  }

  const subscribeChunked = (
    openQuery: EventSubscription,
    cfg: { maxChunkSize?: number; maxChunkTimeMs?: number },
    onChunk: (chunk: EventChunk) => Promise<void> | void,
    onError?: (err: unknown) => void,
  ): CancelSubscription => {
    const { lowerBound, query } = openQuery
    const lb = lowerBound || {}

    const cb = bookKeepingOnChunk(lb, onChunk)

    const bufTime = cfg.maxChunkTimeMs || 5
    const bufSize = cfg.maxChunkSize || 1000
    const s = eventStore.subscribe(lb, query || allEvents)

    const buffered = s
      // 2nd arg to bufferTime is not marked as optional, but it IS optional
      /* eslint-disable @typescript-eslint/no-non-null-assertion */
      .pipe(
        bufferTime(bufTime, null!, bufSize),
        filter((x) => x.length > 0),
        map((buf) => buf.sort(EventKey.ord.compare)),
      )
    /* eslint-enable @typescript-eslint/no-non-null-assertion */

    // The only way to avoid parallel invocations is to use mergeScan with final arg=1
    const rxSub = buffered
      .pipe(mergeScan((_a: void, chunk: Events) => from(cb(chunk)), void 0, 1))
      .subscribe({ error: onError || noop })

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
    onCompleteOrError?: (err?: unknown) => void,
  ): CancelSubscription => {
    const x = subMono(query.sessionId, query.query, query.attemptStartFrom)
      .pipe(
        map((x) => convertMsg<E>(x)),
        mergeScan((_a: void, m: EventsOrTimetravel<E>) => from(Promise.resolve(cb(m))), void 0, 1),
      )
      // The only way to avoid parallel invocations is to use mergeScan with final arg=1
      .subscribe({
        complete: onCompleteOrError || noop,
        error: onCompleteOrError || noop,
      })

    return () => x.unsubscribe()
  }

  // Find first currently known event according to given sorting
  const findFirstKnown = async <E>(
    query: Where<E>,
    order: EventsSortOrder,
  ): Promise<[ActyxEvent<E> | undefined, OffsetMap]> => {
    const cur = await present()

    const firstEvent = await lastValueFrom(
      eventStore.query({}, cur, query, order).pipe(defaultIfEmpty(null), first()),
    )

    return [firstEvent ? wrap(firstEvent) : undefined, cur]
  }

  // Find first currently known event according to an arbitrary decision logic
  const reduceUpToPresent = async <R, E = unknown>(
    query: Where<E>,
    reduce: (acc: R, e1: ActyxEvent<E>) => R,
    initial: R,
  ): Promise<[R, OffsetMap]> => {
    const cur = await present()

    const reducedValue = await lastValueFrom(
      eventStore
        .query(
          {},
          cur,
          query,
          // Doesn't matter, we have to go through all known events anyways
          EventsSortOrder.Ascending,
        )
        .pipe(
          map((e) => wrap<E>(e)),
          rxReduce(reduce, initial),
        ),
    )

    return [reducedValue, cur]
  }

  const callbackWhenReplaced = <E>(
    query: Where<E>,
    startingOffsets: OffsetMap,
    initial: ActyxEvent<E> | undefined,
    onEvent: (event: E, metadata: Metadata) => void,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
    onError?: (err: unknown) => void,
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

    return subscribeChunked({ query, lowerBound: startingOffsets }, {}, cb, onError)
  }

  const observeBestMatch = <E>(
    query: Where<E>,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
    onReplaced: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
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

      cancelSubscription = callbackWhenReplaced(
        query,
        offsets,
        initial,
        onReplaced,
        shouldReplace,
        onError,
      )
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const observeEarliest = <E>(
    tq: EarliestQuery<E>,
    onEvent: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
  ): CancelSubscription => {
    const { query, eventOrder } = tq

    if (eventOrder === EventOrder.Timestamp) {
      return observeBestMatch(query, lt(_ordByTimestamp), onEvent)
    }

    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    /** If lamport order is desired, we can use store-support to speed up the query. */
    findFirstKnown(query, EventsSortOrder.Ascending).then(([earliest, offsets]) => {
      if (cancelled) {
        return
      }

      cancelSubscription = callbackWhenReplaced(
        query,
        offsets,
        earliest,
        onEvent,
        lt(_ordByKey),
        onError,
      )
    })

    return () => {
      cancelled = true
      cancelSubscription && cancelSubscription()
    }
  }

  const observeLatest = <E>(
    tq: LatestQuery<E>,
    onEvent: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
  ): CancelSubscription => {
    const { query, eventOrder } = tq

    if (eventOrder === EventOrder.Timestamp) {
      return observeBestMatch(query, gt(_ordByTimestamp), onEvent)
    }

    let cancelled = false
    let cancelSubscription: CancelSubscription | null = null

    /** If lamport order is desired, we can use store-support to speed up the query. */
    findFirstKnown(query, EventsSortOrder.Descending).then(([latest, offsets]) => {
      if (cancelled) {
        return
      }

      cancelSubscription = callbackWhenReplaced(
        query,
        offsets,
        latest,
        onEvent,
        gt(_ordByKey),
        onError,
      )
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
    onError?: (err: unknown) => void,
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

      cancelSubscription = subscribeChunked({ query, lowerBound: offsets }, {}, cb, onError)
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

    const allPersisted = eventStore.persistEvents(events).pipe(
      toArray(),
      map((x) => x.flat().map(mkMeta)),
      shareReplay(1),
    )

    return pendingEmission(allPersisted)
  }

  // TS doesn’t understand how we are implementing this overload.
  // eslint-disable-next-line @typescript-eslint/ban-ts-comment
  // @ts-ignore
  const publish: EventFns['publish'] = (taggedEvents: ReadonlyArray<TaggedEvent> | TaggedEvent) => {
    if (Array.isArray(taggedEvents)) {
      return emit(taggedEvents).toPromise()
    } else {
      return emit([taggedEvents as TaggedEvent])
        .toPromise()
        .then((x) => x[0])
    }
  }

  // FIXME properly type EventStore. (This runs without error because in production mode the ws event store does not use io-ts.)
  const wrapAql = (e: { type: string }): AqlResponse => {
    const actualType = e.type

    if (actualType === 'offsets' || actualType === 'diagnostic') {
      return e as AqlResponse
    }

    const w = wrap(e as unknown as Event)

    return {
      ...w,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      type: actualType as any,
    }
  }

  const getQueryAndOrd = (query: AqlQuery): [string, EventsSortOrder] => {
    if (isString(query)) {
      return [query, EventsSortOrder.Ascending]
    } else {
      return [query.query, query.order || EventsSortOrder.Ascending]
    }
  }

  const queryAql = async (query: AqlQuery): Promise<AqlResponse[]> => {
    const [aql, ord] = getQueryAndOrd(query)

    return lastValueFrom(eventStore.queryUnchecked(aql, ord).pipe(map(wrapAql), toArray()))
  }
  const subscribeAql = (opts: SubscribeAqlProps): CancelSubscription => {
    const { lowerBound, query, onResponse, onError } = opts
    const lb = lowerBound || {}
    const qr = typeof query === 'string' ? query : query.query

    const rxSub = eventStore
      .subscribeUnchecked(qr, lb)
      .pipe(
        map(wrapAql),
        mergeScan((_a: void, r: AqlResponse) => from(Promise.resolve(onResponse(r))), void 0, 1),
      )
      .subscribe({ error: onError || noop })

    return () => rxSub.unsubscribe()
  }

  const queryAqlChunked = (
    query: AqlQuery,
    chunkSize: number,
    onChunk: (chunk: AqlResponse[]) => Promise<void> | void,
    onCompleteOrError: (err?: unknown) => void,
  ): CancelSubscription => {
    const [aql, ord] = getQueryAndOrd(query)

    const buffered = eventStore.queryUnchecked(aql, ord).pipe(map(wrapAql), bufferCount(chunkSize))

    // The only way to avoid parallel invocations is to use mergeScan with final arg=1
    const rxSub = buffered
      .pipe(
        mergeScan(
          (_a: void, chunk: AqlResponse[]) => from(Promise.resolve(onChunk(chunk))),
          undefined,
          1,
        ),
      )
      .subscribe({ error: onCompleteOrError, complete: onCompleteOrError })

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
    subscribeAql,
    subscribeMonotonic,
    observeEarliest,
    observeLatest,
    observeBestMatch,
    observeUnorderedReduce,
    emit,
    publish,
  }
}
