/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import {
  ActyxEvent,
  AqlResponse,
  CancelSubscription,
  EventChunk,
  EventsOrTimetravel,
  EventsSortOrder,
  FixedStart,
  Metadata,
  OffsetMap,
  OffsetsResponse,
  PendingEmission,
  TaggedEvent,
  TestEvent,
  Where,
} from './types'

/** Which clock to compare events by. Defaults to `Lamport`.
 * @beta */
export enum EventOrder {
  /**
   * Comparison according to Lamport clock, which is a logical clock,
   * meaning it preserves causal order even when wall clocks on devices are off.
   *
   * On the flip-side, for any two events where neither is a cause of the other,
   * lamport-order may be different from timestamp-order, if the devices creating the events
   * where disconnected from each other at the time.
   */
  Lamport = 'lamport',

  /**
   * Comparison according to wall clock time logged at event creation.
   * If the system clock on a device is wrong, the event's timestamp will also be wrong. */
  Timestamp = 'timestamp',
}

/** Query for a fixed set of known events.
 * @public */
export type RangeQuery = {
  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>

  /**
   * Starting point (exclusive) for the query. Everything up-to-and-including `lowerBound` will be omitted from the result. Defaults empty record.
   *
   * Events from sources not included in the `lowerBound` will be delivered from start, IF they are included in `upperBound`.
   * Events from sources missing from both `lowerBound` and `upperBound` will not be delivered at all.
   */
  lowerBound?: OffsetMap

  /**
   * Ending point (inclusive) for the query. Everything covered by `upperBound` (inclusive) will be part of the result.
   *
   * If a source is not included in `upperBound`, its events will not be included in the result.
   **/
  upperBound: OffsetMap

  /** Desired order of delivery. Defaults to 'Asc' */
  order?: EventsSortOrder

  /** Earliest event ID to consider in the result */
  horizon?: string
}

/** Query for a set of events which is automatically capped at the latest available upperBound.
 * @public */
export type AutoCappedQuery = {
  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>

  /** Desired order of delivery. Defaults to 'Asc' */
  order?: EventsSortOrder

  /** Earliest event ID to consider in the result */
  horizon?: string
}

/** Subscription to a set of events that may still grow.
 * @public */
export type EventSubscription = {
  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** Statement to select specific events. Defaults to `allEvents`. */
  query?: Where<unknown>
}

/**
 * Subscribe to a stream of events that will never go backwards in time, but rather terminate with a timetravel-message.
 *
 * @alpha
 */
export type MonotonicSubscription<E> = {
  /** User-chosen session id, used to find cached intermediate states aka local snapshots. */
  sessionId: string

  /** Statement to select specific events. */
  query: Where<E>

  /** Sending 'attemptStartFrom' means we DONT want a snapshot sent as initial message. */
  attemptStartFrom: FixedStart
}

/** Query for observeEarliest.
 * @beta  */
export type EarliestQuery<E> = {
  /** Statement to select specific events. */
  query: Where<E>

  /**
   * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
   * Defaults to empty map, which means no lower bound at all.
   * Sources not listed in the `lowerBound` will be delivered in full.
   */
  lowerBound?: OffsetMap

  /** The order to find min/max for. Defaults to `Lamport`.  */
  eventOrder?: EventOrder
}

/** Query for observeLatest.
 * @beta  */
export type LatestQuery<E> = EarliestQuery<E>

/** An aql query is either a plain string, or an object containing the string and the desired order.
 * @beta */
export type AqlQuery =
  | string
  | {
      /** Query as AQL string */
      query: string

      /** Desired order of delivery (relative to events). Defaults to 'Asc' */
      order?: EventsSortOrder
    }

/**
 * Handler for a streaming operation ending, either normally or with an error.
 * If the `err` argument is defined, the operation completed due to an error.
 * Otherwise, it completed normally.
 * @public
 **/
export type OnCompleteOrErr = (err?: unknown) => void

/** Functions that operate directly on Events.
 * @public  */
export interface EventFns {
  /** Get the current local 'present' i.e. offsets up to which we can provide events without any gaps. */
  present: () => Promise<OffsetMap>

  /** Get the present alongside information on how many events are known to be pending replication from peers to us. */
  offsets: () => Promise<OffsetsResponse>

  /**
   * Get all known events between the given offsets, in one array.
   *
   * @param query       - `RangeQuery` object specifying the desired set of events.
   *
   * @returns A Promise that resolves to the complete set of queries events.
   */
  queryKnownRange: (query: RangeQuery) => Promise<ActyxEvent[]>

  /**
   * Get all known events between the given offsets, in chunks.
   * This is helpful if the result set is too large to fit into memory all at once.
   * The returned `Promise` resolves after all chunks have been delivered.
   *
   * @param query       - `RangeQuery` object specifying the desired set of events.
   * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
   * @param onChunk     - Callback that will be invoked with every chunk, in sequence.
   *
   * @returns A function that can be called in order to cancel the delivery of further chunks.
   */
  queryKnownRangeChunked: (
    query: RangeQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
    onComplete?: OnCompleteOrErr,
  ) => CancelSubscription

  /**
   * Query all known events that occured after the given `lowerBound`.
   *
   * @param query  - `OpenEndedQuery` object specifying the desired set of events.
   *
   * @returns An `EventChunk` with the result and its bounds.
   *          The contained `upperBound` can be passed as `lowerBound` to a subsequent call of this function to achieve exactly-once delivery of all events.
   */
  queryAllKnown: (query: AutoCappedQuery) => Promise<EventChunk>

  /**
   * Query all known events that occured after the given `lowerBound`, in chunks.
   * This is useful if the complete result set is potentially too large to fit into memory at once.
   *
   * @param query       - `OpenEndedQuery` object specifying the desired set of events.
   * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
   * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is an offset map covering all events passed as first arg.
   *
   * @returns A function that can be called in order to cancel the delivery of further chunks.
   */
  queryAllKnownChunked: (
    query: AutoCappedQuery,
    chunkSize: number,
    onChunk: (chunk: EventChunk) => Promise<void> | void,
    onComplete?: OnCompleteOrErr,
  ) => CancelSubscription

  /**
   * Run a custom AQL query and get back the raw responses collected.
   *
   * @param query       - A plain AQL query string.
   *
   * @returns List of all response messages generated by the query.
   *
   * @beta
   */
  queryAql: (query: AqlQuery) => Promise<AqlResponse[]>

  /**
   * Run a custom AQL subscription and get back the raw responses collected via a callback.
   *
   * @param query       - A plain AQL query string.
   * @param onResponse  - Callback that will be invoked for each raw response, in sequence. Even if this is an async function (returning `Promise<void>`), there will be no concurrent invocations of it.
   * @param onError     - Callback that will be invoked in case on a error.
   * @param lowerBound  - Starting point (exclusive) for the query. Everything up-to-and-including `lowerBound` will be omitted from the result. Defaults empty record.
   *
   * @returns A `Promise` that resolves to updated offset-map after all chunks have been delivered.
   *
   * @beta
   */
  subscribeAql: (
    query: AqlQuery,
    onResponse: (r: AqlResponse) => Promise<void> | void,
    onError?: (err: unknown) => void,
    lowerBound?: OffsetMap,
  ) => CancelSubscription

  /**
   * Run a custom AQL query and get the response messages in chunks.
   *
   * @param query       - AQL query
   * @param chunkSize   - Desired chunk size
   * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Even if this is an async function (returning `Promise<void>`), there will be no concurrent invocations of it.
   *
   * @returns A function that can be called in order to cancel the delivery of further chunks.
   *
   * @beta
   */
  queryAqlChunked: (
    query: AqlQuery,
    chunkSize: number,
    onChunk: (chunk: AqlResponse[]) => Promise<void> | void,
    onCompleteOrError: OnCompleteOrErr,
  ) => CancelSubscription

  /**
   * Subscribe to all events fitting the `query` after `lowerBound`.
   *
   * The subscription goes on forever, until manually cancelled.
   *
   * @param query       - `EventSubscription` object specifying the desired set of events.
   * @param onEvent     - Callback that will be invoked for each event, in sequence.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  subscribe: (
    query: EventSubscription,
    onEvent: (e: ActyxEvent) => Promise<void> | void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Subscribe to all events fitting the `query` after `lowerBound`.
   * They will be delivered in chunks of configurable size.
   * Each chunk is internally sorted in ascending `eventId` order.
   * The subscription goes on forever, until manually cancelled.
   *
   * @param query       - `EventSubscription` object specifying the desired set of events.
   * @param chunkConfig - How event chunks should be built.
   * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is the updated offset map.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  subscribeChunked: (
    query: EventSubscription,
    chunkConfig: {
      /** Maximum chunk size. Defaults to 1000. */
      maxChunkSize?: number

      /**
       * Maximum duration (in ms) a chunk of events is allowed to grow, before being passed to the callback.
       * Defaults to 5.
       */
      maxChunkTimeMs?: number
    },
    onChunk: (chunk: EventChunk) => Promise<void> | void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Subscribe to a stream of events until this would go back in time.
   * Instead of going back in time, receive a TimeTravelMsg and terminate the stream.
   *
   * @alpha
   */
  subscribeMonotonic: <E>(
    query: MonotonicSubscription<E>,
    callback: (data: EventsOrTimetravel<E>) => Promise<void> | void,
    onCompleteOrErr?: OnCompleteOrErr,
  ) => CancelSubscription

  /**
   * Observe always the **earliest** event matching the given query.
   * If there is an existing event fitting the query, `onNewEarliest` will be called with that event.
   * Afterwards, `onNewEarliest` will be called whenever a new event becomes known that is older than the previously passed one.
   * Note that the 'earliest' event may keep updating as new events become known.
   *
   * @param query                - Query to select the set of events.
   * @param onNewEarliest        - Callback that will be invoked whenever there is a 'new' earliest event.
   *
   * @returns A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeEarliest: <E>(
    query: EarliestQuery<E>,
    onNewEarliest: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Observe always the **latest** event matching the given query.
   * If there is an existing event fitting the query, `onNewLatest` will be called with that event.
   * Afterwards, `onNewLatest` will be called whenever a new event becomes known that is younger than the previously passed one.
   *
   * @param query                - Query to select the set of events.
   * @param onNewLatest          - Callback that will be invoked for each new latest event.
   *
   * @returns A function that can be called in order to cancel the subscription.
   *
   * @beta
   */
  observeLatest: <E>(
    query: EarliestQuery<E>,
    onNewLatest: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Among all events matching the query, find one that best matches some property.
   * This is useful for finding the event that has `min` or `max` of something.
   * E.g. `shouldReplace = (candidate: ActyxEventy<number>, cur: ActyxEventy<number>) => candidate.payload > cur.payload` keeps finding the event with the highest payload value.
   * Note that there is no guarantee regarding the order in which candidates are passed to the callback!
   * If `shouldReplace(a, b)` returns true, the reversed call `shouldReplace(b, a)` should return false. Otherwise results may be wild.
   *
   * @param query         - Query to select the set of `candidate` events.
   * @param shouldReplace - Should `candidate` replace `cur`?
   * @param onReplaced    - Callback that is evoked whenever replacement happens, i.e. we found a new best match.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  observeBestMatch: <E>(
    query: Where<E>,
    shouldReplace: (candidate: ActyxEvent<E>, cur: ActyxEvent<E>) => boolean,
    onReplaced: (event: E, metadata: Metadata) => void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Apply a `reduce` operation to all events matching `query`, in no specific order.
   * This is useful for operations that are **commutative**, e.g. `sum` or `product`.
   *
   * @param query         - Query to select the set of events to pass to the reducer.
   * @param reduce        - Compute a new state `R` by integrating the next event.
   * @param initial       - Initial, neutral state, e.g. `0` for a `sum` operation.
   * @param onUpdate      - Callback that is evoked with updated results.
   *                        If a batch of events was applied, `onUpdate` will only be called once, with the final new state.
   *
   * @returns A function that can be called in order to cancel the subscription.
   */
  observeUnorderedReduce: <R, E>(
    query: Where<E>,
    reduce: (acc: R, event: E, metadata: Metadata) => R,
    initial: R,
    onUpdate: (result: R) => void,
    onError?: (err: unknown) => void,
  ) => CancelSubscription

  /**
   * Emit a number of events with tags attached.
   *
   * @param events - Events to emit.
   *
   * @returns        A `PendingEmission` object that can be used to register callbacks with the emission’s completion.
   *
   * @deprecated Use `publish` instead, and always await the Promise.
   */
  emit: (events: TaggedEvent[]) => PendingEmission

  /**
   * Publish a number of events with tags attached.
   * This function is the same as `emit`, only it directly returns the Promise.
   *
   * @param events - Events to publish.
   *
   * @returns        A Promise that resolves to the persisted event’s metadata, in the same order they were passed into the function.
   */
  publish(event: TaggedEvent): Promise<Metadata>
  publish(events: TaggedEvent[]): Promise<Metadata[]>
}

/** EventFns for unit-tests.
 * @public */
export type TestEventFns = EventFns & {
  /** Inject an event as if it arrived from anywhere.
   * @public */
  directlyPushEvents: (events: TestEvent[]) => void
}
