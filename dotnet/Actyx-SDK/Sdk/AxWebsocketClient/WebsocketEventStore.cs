using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Reactive.Threading.Tasks;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Actyx.Sdk.Wsrpc;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.AxWebsocketClient
{
    public class WebsocketEventStore : IEventStore
    {
        private readonly WsrpcClient wsrpcClient;
        public NodeId NodeId { get; private set; }
        public string AppId { get; private set; }

        public WebsocketEventStore(WsrpcClient wsrpcClient, NodeId nodeId, string appId)
        {
            this.wsrpcClient = wsrpcClient;
            NodeId = nodeId;
            AppId = appId;

            wsrpcClient.Start();
        }

        public void Dispose()
        {
            wsrpcClient.Dispose();
        }

        public async Task<OffsetsResponse> Offsets() =>
            await wsrpcClient.Request("offsets", null)
                .Take(1)
                .Select(EventStore.Protocol.DeserializeJson<OffsetsResponse>)
                .ToTask();

        public Task<PublishResponse> Publish(IEnumerable<IEventDraft> events)
        {
            if (events is null || events.Count() == 0)
            {
                return Task.FromResult(new PublishResponse { Data = new List<EventPublishMetadata>() });
            }

            var request = new { data = events };
            return wsrpcClient
                .Request("publish", EventStore.Protocol.SerializeJson(request))
                .Take(1)
                .Select(EventStore.Protocol.DeserializeJson<PublishResponse>)
                .ToTask();
        }

        public IObservable<IResponseMessage> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder order)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new
            {
                lowerBound,
                upperBound,
                query = query.ToAql(),
                order,
            };
            return wsrpcClient
                .Request("query", EventStore.Protocol.SerializeJson(request))
                .TrySelect(EventStore.Protocol.DeserializeJson<IResponseMessage>, LogDecodingError);
        }

        public IObservable<IResponseMessage> Subscribe(OffsetMap lowerBound, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new
            {
                lowerBound,
                query = query.ToAql(),
            };
            return wsrpcClient
                .Request("subscribe", EventStore.Protocol.SerializeJson(request))
                .TrySelect(EventStore.Protocol.DeserializeJson<IResponseMessage>, LogDecodingError);
        }

        public IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(string session, OffsetMap startFrom, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(session, nameof(session));
            ThrowIf.Argument.IsNull(startFrom, nameof(startFrom));
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new
            {
                session,
                lowerBound = startFrom,
                query = query.ToAql(),
            };
            return wsrpcClient
                .Request("subscribe_monotonic", EventStore.Protocol.SerializeJson(request))
                .TrySelect(EventStore.Protocol.DeserializeJson<ISubscribeMonotonicResponse>, LogDecodingError);
        }

        private static void LogDecodingError(JToken json, Exception error) =>
            Console.Error.WriteLine($"Error decoding {json}: {error.Message}");
    }
}
