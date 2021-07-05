using System;
using System.Linq;
using System.Collections.Generic;
using System.Reactive.Linq;
using System.Reactive.Threading.Tasks;
using System.Threading.Tasks;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json;
using Actyx.Sdk.Utils.Extensions;
using Actyx.Sdk.Formats;

namespace Actyx
{
    public class WebsocketEventStore : IEventStore, IDisposable
    {
        private readonly WsrpcClient wsrpcClient;
        private readonly string appId;
        private readonly NodeId nodeId;
        private readonly JsonSerializer serializer = JsonSerializer.Create(HttpContentExtensions.JsonSettings);

        public WebsocketEventStore(WsrpcClient wsrpcClient, string appId, NodeId nodeId)
        {
            this.wsrpcClient = wsrpcClient;
            this.appId = appId;
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

        public Task<IEnumerable<EventOnWire>> Publish(IEnumerable<IEventDraft> events) =>
            wsrpcClient
                .Request("publish", JToken.FromObject(new { data = events }, serializer))
                .Take(1)
                .Select(response => response.ToObject<Sdk.AxHttpClient.PublishResponse>(serializer))
                .Select(publishResponse => publishResponse.Data.Zip(events, (metadata, @event) =>
                    new EventOnWire
                    {
                        Lamport = metadata.Lamport,
                        Offset = metadata.Offset,
                        Payload = new JValue(@event.Payload),
                        Stream = metadata.Stream,
                        Tags = @event.Tags,
                        Timestamp = metadata.Timestamp,
                        AppId = appId,
                    })
                ).ToTask();

        public IObservable<IEventOnWire> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder sortOrder) =>
            wsrpcClient
                .Request("query", JToken.FromObject(new
                {
                    lowerBound,
                    upperBound,
                    query = query.ToAql(),
                    order = sortOrder.ToWireString(),
                }, serializer))
                .Select(response => response.ToObject<IEventOnWire>(serializer));

        public IObservable<IEventOnWire> Subscribe(OffsetMap lowerBound, IEventSelection query) =>
            wsrpcClient
                .Request("subscribe", JToken.FromObject(new
                {
                    lowerBound,
                    query = query.ToAql(),
                }, serializer))
                .Select(response => response.ToObject<IEventOnWire>(serializer));
    }
}
