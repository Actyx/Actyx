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

    public class RangeQuery
    {
        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }

        /**
         * Starting point (exclusive) for the query. Everything up-to-and-including `lowerBound` will be omitted from the result. Defaults empty record.
         *
         * Events from sources not included in the `lowerBound` will be delivered from start, IF they are included in `upperBound`.
         * Events from sources missing from both `lowerBound` and `upperBound` will not be delivered at all.
         */
        public OffsetMap LowerBound { get; set; }

        /**
         * Ending point (inclusive) for the query. Everything covered by `upperBound` (inclusive) will be part of the result.
         *
         * If a source is not included in `upperBound`, its events will not be included in the result.
         **/
        public OffsetMap UpperBound { get; set; }

        /** Desired order of delivery. Defaults to 'Asc' */
        public EventsOrder Order { get; set; }
    }

    public class AutoCappedQuery
    {
        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }

        /**
         * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
         * Defaults to empty map, which means no lower bound at all.
         * Sources not listed in the `lowerBound` will be delivered in full.
         */
        public OffsetMap LowerBound { get; set; }

        /** Desired order of delivery. Defaults to 'Asc' */
        public EventsOrder Order { get; set; }
    }

    public class EventSubscription
    {
        /**
         * Starting point for the query. Everything up-to-and-including `lowerBound` will be omitted from the result.
         * Defaults to empty map, which means no lower bound at all.
         * Sources not listed in the `lowerBound` will be delivered in full.
         */
        public OffsetMap LowerBound { get; set; }

        /** Statement to select specific events. Defaults to `allEvents`. */
        public IEventSelection Query { get; set; }

        /** Maximum chunk size. Defaults to 1000. */
        public uint MaxChunkSize { get; set; }

        /**
         * Maximum duration (in ms) a chunk of events is allowed to grow, before being passed to the callback.
         * Defaults to 5.
         * Set this to zero to optimize latency at the cost of always receiving just single events.
         */
        public Nullable<uint> MaxChunkTimeMs { get; set; }
    }

    public class Aql : IEventSelection
    {

        private readonly string aql;

        public Aql(string aql)
        {
            this.aql = aql;
        }

        public string ToAql()
        {
            return this.aql;
        }
    }


    public class Metadata
    {
        // Was this event written by the very node we are running on?
        public bool IsLocalEvent { internal set; get; }

        // Tags belonging to the event.
        public IList<string> Tags { internal set; get; }

        // Time since Unix Epoch **in Microseconds**!
        // FIXME should use dotnet Duration type or something
        public ulong TimestampMicros { internal set; get; }

        // FIXME should offer Dotnet Date type
        //  timestampAsDate: () => Date

        // Lamport timestamp of the event. Cf. https://en.wikipedia.org/wiki/Lamport_timestamp
        public ulong Lamport { internal set; get; }

        // A unique identifier for the event.
        // Every event has exactly one eventId which is unique to it, guaranteed to not collide with any other event.
        // Events are *sorted* based on the eventId by Actyx: For a given event, all later events also have a higher eventId according to simple string-comparison.
        public string EventId { internal set; get; }

        // App id of the event
        public string AppId { internal set; get; }

        // Stream this event belongs to
        public string Stream { internal set; get; }

        // Offset of this event inside its stream
        public ulong Offset { internal set; get; }
    }

    public struct ChunkingOptions
    {
        /** Maximum chunk size. Defaults to 1000, if null */
        public Nullable<uint> MaxChunkSize { get; set; }

        /**
         * Maximum duration (in ms) a chunk of events is allowed to grow, before being passed to the callback.
         * Defaults to 5, if null
         */
        public Nullable<uint> MaxChunkTimeMs { get; set; }
    }

    public struct ActyxEvent
    {
        public Metadata Meta { internal set; get; }
        public JObject Payload { internal set; get; }
    }

    public struct EventChunk
    {
        public OffsetMap LowerBound { internal set; get; }

        public OffsetMap UpperBound { internal set; get; }

        public IList<ActyxEvent> Events { internal set; get; }
    }

    public interface IEventFns
    {
        public Task<OffsetMap> Present();

        public Task<OffsetsResponse> Offsets();

        /**
         * Get all known events between the given offsets, in one array.
         *
         * @param query       - `RangeQuery` object specifying the desired set of events.
         *
         * @returns A Promise that resolves to the complete set of queries events.
         */
        public Task<IList<ActyxEvent>> QueryKnownRange(RangeQuery query);

        /**
         * Get all known events between the given offsets, in chunks.
         * This is helpful if the result set is too large to fit into memory all at once.
         * The returned `Promise` resolves after all chunks have been delivered.
         *
         * @param query       - `RangeQuery` object specifying the desired set of events.
         * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
         * @param onChunk     - Callback that will be invoked with every chunk, in sequence.
         *
         * @returns A Promise that resolves when all chunks have been delivered to the callback.
         */
        public IObservable<EventChunk> QueryKnownRangeChunked(
            RangeQuery query,
            uint chunkSize
        );

        /**
         * Query all known events that occurred after the given `lowerBound`.
         *
         * @param query  - `OpenEndedQuery` object specifying the desired set of events.
         *
         * @returns An `EventChunk` with the result and its bounds.
         *          The contained `upperBound` can be passed as `lowerBound` to a subsequent call of this function to achieve exactly-once delivery of all events.
         */
        public Task<EventChunk> QueryAllKnown(AutoCappedQuery query);

        /**
         * Query all known events that occurred after the given `lowerBound`, in chunks.
         * This is useful if the complete result set is potentially too large to fit into memory at once.
         *
         * @param query       - `OpenEndedQuery` object specifying the desired set of events.
         * @param chunkSize   - Maximum size of chunks. Chunks may be smaller than this.
         * @param onChunk     - Callback that will be invoked for each chunk, in sequence. Second argument is an offset map covering all events passed as first arg.
         *
         * @returns A `Promise` that resolves to updated offset-map after all chunks have been delivered.
         */
        public IObservable<EventChunk> QueryAllKnownChunked(
            AutoCappedQuery query,
            ulong chunkSize
        );

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
        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub);
        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub, ChunkingOptions chunkConfig);


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
        public IObservable<ActyxEvent> Subscribe(EventSubscription sub);
    }
}
