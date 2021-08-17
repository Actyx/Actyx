using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;

namespace Actyx.Sdk.AxHttpClient
{
    public class HttpEventStore : IEventStore
    {
        private readonly IAxHttpClient client;
        public NodeId NodeId { get; private set; }
        public string AppId { get; private set; }

        public HttpEventStore(IAxHttpClient client, NodeId nodeId, string appId)
        {
            this.client = client;
            NodeId = nodeId;
            AppId = appId;
        }

        public async Task<OffsetsResponse> Offsets() =>
            await client.Get<OffsetsResponse>("offsets");

        public async Task<PublishResponse> Publish(IEnumerable<IEventDraft> events)
        {
            if (events is null || events.Count() == 0)
            {
                return new PublishResponse { Data = new List<EventPublishMetadata>() };
            }

            return await client.Post<object, PublishResponse>("publish", new { data = events });
        }

        public IObservable<IResponseMessage> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder order)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));
            ThrowIf.Argument.IsNull(order, nameof(order));

            var request = new { lowerBound, upperBound, query = query.ToAql(), order };
            return client.Stream<object, IResponseMessage>("query", request);
        }

        public IObservable<IResponseMessage> Subscribe(OffsetMap lowerBound, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new { lowerBound, query = query.ToAql() };
            return client.Stream<object, IResponseMessage>("subscribe", request);
        }

        public IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(string session, OffsetMap startFrom, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(session, nameof(session));
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new { session, lowerBound = startFrom, query = query.ToAql() };
            return client.Stream<object, ISubscribeMonotonicResponse>("subscribe_monotonic", request);
        }

        public void Dispose()
        {
            // Nothing to do?
        }
    }
}
