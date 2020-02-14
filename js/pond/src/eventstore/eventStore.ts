import { Observable } from 'rxjs'
import { SubscriptionSet } from '../subscription'
import { EventKey, SourceId, Milliseconds } from '../types'
import { mockEventStore } from './mockEventStore'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { testEventStore } from './testEventStore'
import {
  AllEventsSortOrder,
  ConnectivityStatus,
  Events,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  UnstoredEvents,
} from './types'
import { WebsocketEventStore } from './websocketEventStore'

/**
 * Get the store's source id.
 */
export type RequestSourceId = () => Observable<SourceId>

/**
 * Get the store's connectivity status
 */
export type RequestConnectivity = (
  specialSources: ReadonlyArray<SourceId>,
  hbHistDelayMicros: number,
  reportEvery: Milliseconds,
  currentPsnHistoryDelay: number,
) => Observable<ConnectivityStatus>

/**
 * Request the full present of the store, so the maximum psn for each source that the store has seen and ingested.
 */
export type RequestPresent = () => Observable<OffsetMapWithDefault>

/**
 * Request the highest seen offsets from the store, not all of which may be available (gapless) yet.
 */
export type RequestHighestSeen = () => Observable<OffsetMapWithDefault>

/**
 * This method is only concerned with already persisted events, so it will always return a finite (but possibly large)
 * stream. It does not make sense to have a toPsn that is in the future, however it might be convenient to provide
 * a toPsn that is unlimited and get back all events up to the present at the time of invocation without needing an
 * additional call to present.
 *
 * The returned event chunks can contain events from multiple sources. If the sort order is unsorted, we guarantee
 * that chunks will be sorted by ascending psn for each source. If the sort order is by event key, we guarantee(*)
 * the same except if the timestamps are non-monotonic for a source, which should not happen usually.
 *
 * Looking for a semantic snapshot can be accomplished by getting events in reverse event key order and aborting the
 * iteration as soon as a semantic snapshot is found.
 *
 * Depending on latency between pond and store the store will traverse into the past a bit further than needed, but
 * given that store and pond are usually on the same machine this won't be that bad, and in any case this is perferable
 * to needing a way of sending a javascript predicate to the store.
 */
export type RequestPersistedEvents = (
  fromPsnsExcluding: OffsetMapWithDefault, // FIXME what does from/to mean when we reverse? would min/max be better?
  toPsnsIncluding: OffsetMapWithDefault,
  subscriptionSet: SubscriptionSet,
  sortOrder: PersistedEventsSortOrder,
  minEventKey?: EventKey,
) => Observable<Events>

/**
 * This method is concerned with both persisted and future events, so it can return an infinite stream if the toPsn
 * is in the future.
 *
 * The returned event chunks can contain events from multiple sources. If the sort order is unsorted, we guarantee
 * that chunks will be sorted by ascending psn for each source. If the sort order is by event key, we guarantee(*)
 * the same except if the timestamps are non-monotonic for a source, which should not happen usually.
 *
 * A sort order of anything else but unsorted only makes sense if either fromPsnsExcluding or toPsnsIncluding
 * constrains the set of sources to a finite set. When asking for sorted events for an infinite set of sources,
 * the returned observable will immediately error out. (Error NOT YET IMPLEMENTED, works with infinite sources.)
 *
 * When having a finite set of sources, sorting by event key can be used to get events in such a way that there
 * will be no time travel. (NOT YET IMPLEMENTED)
 *
 * Getting events up to a maximum event key can be achieved for a finite set of sources by specifying sort by
 * event key and aborting as soon as the desired event key is reached.
 */
export type RequestAllEvents = (
  fromPsnsExcluding: OffsetMapWithDefault, // FIXME what does from/to mean when we reverse? would min/max be better?
  toPsnsIncluding: OffsetMapWithDefault,
  subscriptionSet: SubscriptionSet,
  sortOrder: AllEventsSortOrder,
  minEventKey?: EventKey,
) => Observable<Events>

/**
 * Store the events in the store and return them as generic events.
 */
export type RequestPersistEvents = (events: UnstoredEvents) => Observable<Events>

/**
 * publish a log message via the event store.
 */

export type EventStore = {
  readonly sourceId: SourceId
  readonly present: RequestPresent
  readonly highestSeen: RequestHighestSeen
  readonly persistedEvents: RequestPersistedEvents
  readonly allEvents: RequestAllEvents
  readonly persistEvents: RequestPersistEvents
  readonly connectivityStatus: RequestConnectivity
}

const noopEventStore: EventStore = {
  allEvents: () => Observable.empty(),
  persistedEvents: () => Observable.empty(),
  present: () => Observable.of({ psns: {}, default: 'max' as 'max' }),
  highestSeen: () => Observable.of({ psns: {}, default: 'max' as 'max' }),
  persistEvents: () => Observable.empty(),
  sourceId: SourceId.of('NoopSourceId'),
  connectivityStatus: () => Observable.empty(),
}

export const EventStore = {
  noop: noopEventStore,
  ws: (multiplexedWebsocket: MultiplexedWebsocket, sourceId: SourceId) =>
    new WebsocketEventStore(multiplexedWebsocket, sourceId),
  mock: mockEventStore,
  test: testEventStore,
}
