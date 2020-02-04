import * as t from 'io-ts'
import { equals, partition } from 'ramda'
import { Observable } from 'rxjs'
import log from '../loggers'
import { SubscriptionSetIO } from '../subscription'
import { EventKey, EventKeyIO, Lamport, Psn, Semantics, SourceId } from '../types'
import {
  EventStore,
  RequestAllEvents,
  RequestPersistedEvents,
  RequestPersistEvents,
  RequestPresent,
} from './eventStore'
import { MultiplexedWebsocket, validateOrThrow } from './multiplexedWebsocket'
import {
  AllEventsSortOrder,
  Event,
  Events,
  OffsetMapWithDefault,
  PersistedEventsSortOrder,
  UnstoredEvents,
} from './types'

export const enum RequestTypes {
  SourceId = '/ax/events/getSourceId',
  Present = '/ax/events/requestPresent',
  PersistedEvents = '/ax/events/requestPersistedEvents',
  AllEvents = '/ax/events/requestAllEvents',
  PersistEvents = '/ax/events/persistEvents',
}
const EventKeyOrNull = t.union([t.null, EventKeyIO])
const ValueOrLimit = t.union([t.number, t.literal('min'), t.literal('max')])
export type ValueOrLimit = t.TypeOf<typeof ValueOrLimit>
export const PersistedEventsRequest = t.readonly(
  t.type({
    minEventKey: EventKeyOrNull,
    fromPsnsExcluding: OffsetMapWithDefault,
    toPsnsIncluding: OffsetMapWithDefault,
    subscriptionSet: SubscriptionSetIO,
    sortOrder: PersistedEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type PersistedEventsRequest = t.TypeOf<typeof PersistedEventsRequest>

export const AllEventsRequest = t.readonly(
  t.type({
    fromPsnsExcluding: OffsetMapWithDefault,
    minEventKey: EventKeyOrNull,
    toPsnsIncluding: OffsetMapWithDefault,
    subscriptionSet: SubscriptionSetIO,
    sortOrder: AllEventsSortOrder,
    count: ValueOrLimit,
  }),
)
export type AllEventsRequest = t.TypeOf<typeof AllEventsRequest>

export const PersistEventsRequest = t.readonly(t.type({ events: UnstoredEvents }))
export type PersistEventsRequest = t.TypeOf<typeof PersistEventsRequest>

export const getSourceId = (multiplexedWebsocket: MultiplexedWebsocket): Promise<SourceId> =>
  multiplexedWebsocket
    .request(RequestTypes.SourceId)
    .map(validateOrThrow(SourceId.FromString))
    .first()
    .toPromise()

export class WebsocketEventStore implements EventStore {
  /* Regarding jelly fish, cf. https://github.com/Actyx/Cosmos/issues/2797 */
  jellyPsn = Psn.zero
  private _present: Observable<OffsetMapWithDefault>
  constructor(private readonly multiplexer: MultiplexedWebsocket, readonly sourceId: SourceId) {
    this._present = Observable.defer(() =>
      this.multiplexer.request(RequestTypes.Present).map(validateOrThrow(OffsetMapWithDefault)),
    ).shareReplay(1)
  }

  present: RequestPresent = () => this._present

  persistedEvents: RequestPersistedEvents = (
    fromPsnsExcluding,
    toPsnsIncluding,
    subscriptionSet,
    sortOrder,
    minEventKey,
  ) => {
    const minEvKey =
      minEventKey === undefined || equals(minEventKey, EventKey.zero) ? null : minEventKey
    return this.multiplexer
      .request(
        RequestTypes.PersistedEvents,
        PersistedEventsRequest.encode({
          fromPsnsExcluding,
          toPsnsIncluding,
          subscriptionSet,
          minEventKey: minEvKey,
          sortOrder,
          count: 'max',
        }),
      )
      .map(validateOrThrow(Events))
  }

  allEvents: RequestAllEvents = (
    fromPsnsExcluding,
    toPsnsIncluding,
    subscriptionSet,
    sortOrder,
    minEventKey,
  ) => {
    const minEvKey =
      minEventKey === undefined || equals(minEventKey, EventKey.zero) ? null : minEventKey
    return this.multiplexer
      .request(
        RequestTypes.AllEvents,
        AllEventsRequest.encode({
          fromPsnsExcluding,
          toPsnsIncluding,
          subscriptionSet,
          minEventKey: minEvKey,
          sortOrder,
          count: 'max',
        }),
      )
      .map(validateOrThrow(Events))
  }

  persistEvents: RequestPersistEvents = events => {
    // extract jelly events, as they must not be sent
    // over the wire to the store
    const [jellyEvents, publishEvents] = partition(
      ({ semantics }) => Semantics.isJelly(semantics),
      events,
    )
    // add source and fake psn to jelly events
    const jellyEventsFromStore: Events = jellyEvents.map(e => ({
      ...e,
      lamport: Lamport.of(this.jellyPsn),

      psn: Psn.of(this.jellyPsn++),
      sourceId: this.sourceId,
    }))

    return publishEvents.length === 0
      ? Observable.of(jellyEventsFromStore)
      : this.multiplexer
          .request(
            RequestTypes.PersistEvents,
            PersistEventsRequest.encode({ events: publishEvents }),
          )
          .map(validateOrThrow(t.type({ events: Events })))
          .map(({ events: persistedEvents }) => {
            if (publishEvents.length !== persistedEvents.length) {
              log.ws.error(
                'PutEvents: Sent %d events, but only got %d PSNs back.',
                publishEvents.length,
                events.length,
              )
              return []
            }
            return publishEvents
              .map<Event>((ev, idx) => ({
                sourceId: this.sourceId,
                name: ev.name,
                payload: ev.payload,
                semantics: ev.semantics,
                timestamp: persistedEvents[idx].timestamp,
                lamport: persistedEvents[idx].lamport,
                psn: persistedEvents[idx].psn,
              }))
              .concat(jellyEventsFromStore)
          })
          .defaultIfEmpty([])
  }
}
