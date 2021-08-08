using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
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
        private readonly JsonContentConverter converter;
        public NodeId NodeId => client.NodeId;

        public HttpEventStore(IAxHttpClient client, JsonContentConverter converter)
        {
            this.client = client;
            this.converter = converter;
        }

        public async Task<OffsetsResponse> Offsets()
        {
            var response = await client.Get(HttpApiPath.OFFSETS_SEG);
            return await converter.FromContent<OffsetsResponse>(response.Content);
        }

        public async Task<PublishResponse> Publish(IEnumerable<IEventDraft> events)
        {
            if (events is null || events.Count() == 0)
            {
                return new PublishResponse { Data = new List<EventPublishMetadata>() };
            }

            var response = await client.Post(HttpApiPath.PUBLISH_SEG, new { data = events });
            return await converter.FromContent<PublishResponse>(response.Content);
        }

        public IObservable<IResponseMessage> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder order)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new { lowerBound, upperBound, query = query.ToAql(), order };
            return Observable
                .FromAsync(() => client.Post(HttpApiPath.QUERY_SEG, request, true))
                .SelectMany(ResponseMessages<IResponseMessage>);
        }

        public IObservable<IResponseMessage> Subscribe(OffsetMap lowerBound, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new { lowerBound, query = query.ToAql() };
            return Observable
                .FromAsync(() => client.Post(HttpApiPath.SUBSCRIBE_SEG, request, true))
                .SelectMany(ResponseMessages<IResponseMessage>);
        }

        public IObservable<ISubscribeMonotonicResponse> SubscribeMonotonic(string session, OffsetMap startFrom, IEventSelection query)
        {
            ThrowIf.Argument.IsNull(session, nameof(session));
            ThrowIf.Argument.IsNull(query, nameof(query));

            var request = new { session, lowerBound = startFrom, query = query.ToAql() };
            return Observable
                .FromAsync(() => client.Post(HttpApiPath.SUBSCRIBE_MONOTONIC_SEG, request, true))
                .SelectMany(ResponseMessages<ISubscribeMonotonicResponse>);
        }

        private IObservable<T> ResponseMessages<T>(HttpResponseMessage response) =>
            Observable
                .FromAsync(async () => await response.EnsureSuccessStatusCodeCustom())
                .SelectMany(_ => response.Content!
                    .ReadFromNdjsonAsync().ToObservable()
                    .TrySelect(EventStore.Protocol.DeserializeJson<T>, LogDecodingError));

        private static void LogDecodingError(JToken json, Exception error) =>
            Console.Error.WriteLine($"Error decoding {json}: {error.Message}");

        public void Dispose()
        {
            // Nothing to do?
        }
    }
}
