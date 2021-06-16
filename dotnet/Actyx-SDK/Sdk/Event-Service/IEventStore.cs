using System;
using System.Collections.Generic;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System.Reactive;
using System.Threading;
using System.Threading.Tasks;

namespace Actyx
{
    // Internal event class, 1:1 correspondence with wire format
    class EventOnWire
    {
        public ulong Lamport { get; set; }

        public ulong Offset { get; set; }

        public ulong Timestamp { get; set; }

        public string Stream { get; set; }

        public string AppId { get; set; }

        public List<string> Tags { get; set; }

        public JObject Payload { get; set; }
    }

    // This interface is not public, it is the internal adapter for switching between ws/http/test impl.
    interface IEventStore
    {

        /**
         * Request the full present of the store, so the maximum CONTIGUOUS offset for each source that the store has seen and ingested.
         * The store will NEVER deliver events across PSN gaps. So the 'present' signifies that which the store is willing to deliver to us.
         * If Offset=2 of some source never reaches our store, that source’s present will never progress beyond Offset=1 for our store.
         * Nor will it expose us to those events that lie after the gap.
         * This also returns the events per source which are pending replication to this node.
         */
        Task<OffsetsReponse> offsets();

        /**
         * This method is only concerned with already persisted events, so it will always return a finite (but possibly large)
         * stream.
         * It is an ERROR to query with unbounded or future PSN.
         *
         * The returned event chunks can contain events from multiple sources. If the sort order is unsorted, we guarantee
         * that chunks will be sorted by ascending psn for each source.
         *
         * Looking for a semantic snapshot can be accomplished by getting events in reverse event key order and aborting the
         * iteration as soon as a semantic snapshot is found.
         *
         * Depending on latency between pond and store the store will traverse into the past a bit further than needed, but
         * given that store and pond are usually on the same machine this won't be that bad, and in any case this is perferable
         * to needing a way of sending a javascript predicate to the store.
         */
        IObservable<EventOnWire> query(
            OffsetMap lowerBound,
            OffsetMap upperBound,
            IEventSelection query,
            EventsOrder sortOrder
        );

        /**
         * This method is concerned with both persisted and future events, so it will always return an infinite stream.
         *
         * The returned event chunks can contain events from multiple sources. Any individual source will not time-travel.
         * There is not sorting between different sources, not even within a single chunk.
         *
         * Getting events up to a maximum event key can be achieved for a finite set of sources by specifying sort by
         * event key and aborting as soon as the desired event key is reached.
         */
        IObservable<EventOnWire> subscribe(
            OffsetMap lowerBound,
            IEventSelection query
        );

        /**
         * Store the events in the store and return them as generic events.
         */
        IObservable<EventOnWire> persistEvents(
            IEnumerable<IEventDraft> events
        );
    }

}
