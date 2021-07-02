using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
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

        public async Task<IEnumerable<EventOnWire>> Publish(IEnumerable<IEventDraft> events)
        {
            if (events is null || events.Count() == 0)
            {
                return Enumerable.Empty<EventOnWire>();
            }

            var response = await client.Post(HttpApiPath.PUBLISH_SEG, new { data = events });

            var publishResponse = await response.Content.ReadFromJsonAsync<PublishResponse>();
            if (publishResponse.Data.Count() != events.Count())
            {
                throw new Exception("Sent event count differs from returned metadata count");
            }

            return publishResponse.Data.Zip(events, (metadata, ev) =>
                new EventOnWire
                {
                    Lamport = metadata.Lamport,
                    Offset = metadata.Offset,
                    Payload = new JValue(ev.Payload),
                    Stream = metadata.Stream,
                    Tags = ev.Tags,
                    Timestamp = metadata.Timestamp,
                    AppId = client.AppId,
                });
        }

        public IObservable<IEventOnWire> Query(OffsetMap lowerBound, OffsetMap upperBound, IEventSelection query, EventsOrder order)
        {
            if (query is null)
            {
                throw new ArgumentNullException(nameof(query));
            }

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
            if (query is null)
            {
                throw new ArgumentNullException(nameof(query));
            }

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
    }

}
