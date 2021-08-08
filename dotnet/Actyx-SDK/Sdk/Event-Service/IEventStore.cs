using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Wsrpc;

namespace Actyx
{
    public static class EventStore
    {
        public static readonly JsonProtocol Protocol = new(EventStoreSerializer.Create());

        public async static Task<IEventStore> Create(AppManifest manifest, ActyxOpts options)
        {
            ThrowIf.Argument.IsNull(options, nameof(options));

            string basePath = $"{options.Host}:{options.Port}/api/v2/";
            var converter = new JsonContentConverter(EventStoreSerializer.Create());
            var axHttpClient = await AxHttpClient.Create($"http://{basePath}", manifest, converter);

            if (options.Transport == Transport.Http)
            {
                return new HttpEventStore(axHttpClient, converter);
            }

            Uri axWs = new($"ws://{basePath}events?{axHttpClient.Token}");
            var wsrpcClient = new WsrpcClient(axWs);
            return new WebsocketEventStore(wsrpcClient, axHttpClient.NodeId);
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
        IObservable<IResponseMessage> Query(
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
        IObservable<IResponseMessage> Subscribe(
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
