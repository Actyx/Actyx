using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive;
using System.Reactive.Linq;
using System.Threading;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx
{
    public class EventFunctions : IEventFns
    {
        private readonly IEventStore store;
        public EventFunctions(IEventStore store)
        {
            this.store = store;
        }

        public Task<OffsetsResponse> Offsets() => store.Offsets();


        public async Task<OffsetMap> Present()
        {
            var result = await store.Offsets();
            return result.Present;
        }

        public async Task<EventChunk> QueryAllKnown(AutoCappedQuery query)
        {
            var wireEvents = await QueryKnown(new RangeQuery
            {
                LowerBound = query.LowerBound,
                Order = query.Order,
                Query = query.Query,
                UpperBound = await Present(),
            });
            var events = wireEvents.OfType<EventOnWire>().Select(ActyxEvent.From(store.NodeId)).ToList();
            var offset = wireEvents.OfType<OffsetsOnWire>().Last();

            return new EventChunk(query.LowerBound, offset.Offsets, events);
        }

        public IObservable<EventChunk> QueryAllKnownChunked(AutoCappedQuery query, uint chunkSize) =>
            Observable.FromAsync(() => Present()).SelectMany(upperBound => QueryKnownRangeChunked(new RangeQuery
            {
                LowerBound = query.LowerBound,
                Order = query.Order,
                Query = query.Query,
                UpperBound = upperBound,
            }, chunkSize));


        public async Task<IList<ActyxEvent>> QueryKnownRange(RangeQuery query)
        {
            var wireEvents = await QueryKnown(query);
            var events = wireEvents.OfType<EventOnWire>().Select(ActyxEvent.From(store.NodeId)).ToList();

            return events;
        }

        public IObservable<EventChunk> QueryKnownRangeChunked(RangeQuery query, uint chunkSize)
        {
            throw new NotImplementedException();
        }

        public IObservable<ActyxEvent> Subscribe(EventSubscription sub)
        {
            throw new NotImplementedException();
        }

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub)
        {
            throw new NotImplementedException();
        }

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub, ChunkingOptions chunkConfig)
        {
            throw new NotImplementedException();
        }

        private async Task<IEnumerable<IEventOnWire>> QueryKnown(RangeQuery query)
        {
            var wireEvents = await store.Query(
                query.LowerBound,
                query.UpperBound,
                query.Query ?? SelectAllEvents.Instance,
                query.Order
            ).ToList();

            return wireEvents;
        }
    }
}
