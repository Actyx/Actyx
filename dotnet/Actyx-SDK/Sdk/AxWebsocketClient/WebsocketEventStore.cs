using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Reactive.Threading.Tasks;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.AxWebsocketClient
{
    public class WebsocketEventStore : IEventStore
    {
        private readonly WsrpcClient wsrpcClient;
        private readonly NodeId nodeId;
        private readonly JsonSerializer serializer = JsonSerializer.Create(HttpContentExtensions.JsonSettings);

        public WebsocketEventStore(WsrpcClient wsrpcClient, NodeId nodeId)
        {
            this.wsrpcClient = wsrpcClient;
            this.nodeId = nodeId;
            wsrpcClient.Start();
        }

        public NodeId NodeId => nodeId;

        public void Dispose()
        {
            wsrpcClient.Dispose();
        }

        public async Task<OffsetsResponse> Offsets() =>
            await wsrpcClient.Request("offsets", null)
                .Take(1)
                .Select(offsets => offsets.ToObject<OffsetsResponse>(serializer))
                .ToTask();

        public Task<PublishResponse> Publish(IEnumerable<IEventDraft> events) =>
            wsrpcClient
                .Request("publish", JToken.FromObject(new { data = events }, serializer))
                .Take(1)
                .Select(response => response.ToObject<PublishResponse>(serializer))
                .ToTask();

        public IObservable<IEventOnWire> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder sortOrder)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            return wsrpcClient
                 .Request("query", JToken.FromObject(new
                 {
                     lowerBound,
                     upperBound,
                     query = query.ToAql(),
                     order = sortOrder.ToWireString(),
                 }, serializer))
                 .Select(response => response.ToObject<IEventOnWire>(serializer));
        }

        public IObservable<IEventOnWire> Subscribe(OffsetMap lowerBound, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            return wsrpcClient
                   .Request("subscribe", JToken.FromObject(new
                   {
                       lowerBound,
                       query = query.ToAql(),
                   }, serializer))
                   .Select(response => response.ToObject<IEventOnWire>(serializer));
        }

        public IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(string session, OffsetMap startFrom, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(session, nameof(session));
            ThrowIf.Argument.IsNull(startFrom, nameof(startFrom));
            ThrowIf.Argument.IsNull(query, nameof(query));

            return wsrpcClient
                .Request("subscribe_monotonic", JToken.FromObject(new
                {
                    session,
                    lowerBound = startFrom,
                    query = query.ToAql(),
                }, serializer))
                .Select(response => response.ToObject<ISubscribeMonotonicResponse>(serializer));
        }
    }
}
