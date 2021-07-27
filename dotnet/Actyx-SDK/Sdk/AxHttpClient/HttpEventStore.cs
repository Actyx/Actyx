using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.AxHttpClient
{
    public class HttpEventStore : IEventStore
    {
        private readonly IAxHttpClient client;

        public HttpEventStore(IAxHttpClient client)
        {
            this.client = client;
        }

        public NodeId NodeId => client.NodeId;

        public async Task<OffsetsResponse> Offsets()
        {
            var response = await client.Get(HttpApiPath.OFFSETS_SEG);
            return await response.Content.ReadFromJsonAsync<OffsetsResponse>();
        }

        public async Task<PublishResponse> Publish(IEnumerable<IEventDraft> events)
        {
            if (events is null || events.Count() == 0)
            {
                return new PublishResponse { Data = new List<EventPublishMetadata>() };
            }

            var response = await client.Post(HttpApiPath.PUBLISH_SEG, new { data = events });

            return await response.Content.ReadFromJsonAsync<PublishResponse>();
        }

        public IObservable<IEventOnWire> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder order)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            return Observable.FromAsync(() => client.Post(HttpApiPath.QUERY_SEG, new
            {
                lowerBound,
                upperBound,
                query = query.ToAql(),
                order = order.ToWireString(),
            }, true)).SelectMany(response =>
            {
                response.EnsureSuccessStatusCode();
                return response.Content!.ReadFromNdjsonAsync<IEventOnWire>().ToObservable();
            });
        }

        public IObservable<IEventOnWire> Subscribe(OffsetMap lowerBound, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            return Observable.FromAsync(() => client.Post(HttpApiPath.SUBSCRIBE_SEG, new
            {
                lowerBound,
                query = query.ToAql(),
            }, true)).SelectMany(response =>
            {
                response.EnsureSuccessStatusCode();
                return response.Content!.ReadFromNdjsonAsync<IEventOnWire>().ToObservable();
            });
        }

        public IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(string session, OffsetMap startFrom, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(session, nameof(session));
            ThrowIf.Argument.IsNull(query, nameof(query));

            return Observable.FromAsync(() => client.Post(HttpApiPath.SUBSCRIBE_MONOTONIC_SEG, new
            {
                session,
                lowerBound = startFrom,
                query = query.ToAql(),
            }, true)).SelectMany(response =>
            {
                response.EnsureSuccessStatusCode();
                return response.Content!.ReadFromNdjsonAsync<ISubscribeMonotonicResponse>().ToObservable();
            });
        }

        public void Dispose()
        {
            // Nothing to do?
        }
    }

}
