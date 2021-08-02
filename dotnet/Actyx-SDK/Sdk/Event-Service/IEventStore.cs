using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Newtonsoft.Json.Linq;

namespace Actyx
{
    public class EventPublishMetadata
    {
        public ulong Lamport { get; set; }
        public ulong Offset { get; set; }
        public ulong Timestamp { get; set; }
        public string Stream { get; set; }
    }

    public interface IEventOnWire { }

    public class OffsetsOnWire : IEventOnWire
    {
        public OffsetMap Offsets { get; set; }
    }

    // Internal event class, 1:1 correspondence with wire format
    public class EventOnWire : EventPublishMetadata, IEventOnWire
    {
        public string AppId { get; set; }
        public IEnumerable<string> Tags { get; set; }
        public JToken Payload { get; set; }
    }

    public interface ISubscribeMonotonicResponse { }

    public class SubscribeMonotonicOffsetsResponse : ISubscribeMonotonicResponse
    {
        public OffsetMap Offsets { get; set; }
    }

    public class SubscribeMonotonicEventResponse : ISubscribeMonotonicResponse
    {
        public string AppId { get; set; }
        public IEnumerable<string> Tags { get; set; }
        public JToken Payload { get; set; }
        public ulong Lamport { get; set; }
        public ulong Offset { get; set; }
        public ulong Timestamp { get; set; }
        public string Stream { get; set; }
        public bool CaughtUp { get; set; }
    }

    public class SubscribeMonotonicTimeTravelResponse : ISubscribeMonotonicResponse
    {
        public EventKey NewStart { get; set; }
    }

    public class PublishResponse
    {
        public IEnumerable<EventPublishMetadata> Data { get; set; }
    }

    public static class EventStore
    {
        public async static Task<IEventStore> Create(AppManifest manifest, ActyxOpts options)
        {
            ThrowIf.Argument.IsNull(options, nameof(options));

            string basePath = $"{options.Host}:{options.Port}/api/v2/";

            if (options.Transport == Transport.Http)
            {
                return new HttpEventStore(await AxHttpClient.Create($"http://{basePath}", manifest));
            }

            Uri axHttp = new($"http://{basePath}");
            var token = await AxHttpClient.GetToken(axHttp, manifest);
            var nodeId = await AxHttpClient.GetNodeId(axHttp);
            Uri axWs = new($"ws://{basePath}events?{token.Token}");
            var wsrpcClient = new WsrpcClient(axWs);
            return new WebsocketEventStore(wsrpcClient, nodeId);
        }
    }

    // This interface is not public, it is the internal adapter for switching between ws/http/test impl.
    public interface IEventStore : IDisposable
    {
        NodeId NodeId { get; }

        /**
         * Request the full present of the store, so the maximum CONTIGUOUS offset for each source that the store has seen and ingested.
         * The store will NEVER deliver events across PSN gaps. So the 'present' signifies that which the store is willing to deliver to us.
         * If Offset=2 of some source never reaches our store, that source’s present will never progress beyond Offset=1 for our store.
         * Nor will it expose us to those events that lie after the gap.
         * This also returns the events per source which are pending replication to this node.
         */
        Task<OffsetsResponse> Offsets();

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
        IObservable<IEventOnWire> Query(
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
        IObservable<IEventOnWire> Subscribe(
            OffsetMap lowerBound,
            IEventSelection query
        );

        IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(
            string session,
            OffsetMap startFrom,
            IEventSelection query
        );

        /**
         * Store the events in the store and return them as generic events.
         */
        Task<PublishResponse> Publish(
            IEnumerable<IEventDraft> events
        );
    }

}
