using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive;
using System.Reactive.Linq;
using System.Threading;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
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

        public IObservable<EventChunk> QueryAllKnownChunked(AutoCappedQuery query, int chunkSize) =>
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

        public IObservable<EventChunk> QueryKnownRangeChunked(RangeQuery query, int chunkSize)
        {
            ThrowIf.Argument.IsNull(query, nameof(query));
            ThrowIf.Argument.IsNull(query.UpperBound, nameof(query.UpperBound));

            return store
                .Query(
                    query.LowerBound ?? new OffsetMap(),
                    query.UpperBound,
                    query.Query ?? SelectAllEvents.Instance,
                    query.Order
                )
                .OfType<EventOnWire>()
                .Select(ActyxEvent.From(store.NodeId))
                .Buffer(chunkSize)
                .Select(query.Order == EventsOrder.Asc ? BookKeepingOnChunk(query.LowerBound) : ReverseBookKeepingOnChunk(query.UpperBound));
        }

        public IObservable<ActyxEvent> Subscribe(EventSubscription sub) => store
            .Subscribe(sub.LowerBound ?? new OffsetMap(), sub.Query ?? SelectAllEvents.Instance)
            .OfType<EventOnWire>()
            .Select(ActyxEvent.From(store.NodeId));

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub) =>
            SubscribeChunked(sub, new ChunkingOptions { MaxChunkSize = 1000, MaxChunkTime = TimeSpan.FromMilliseconds(5) });

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub, ChunkingOptions chunkConfig) =>
             store
                .Subscribe(sub.LowerBound ?? new OffsetMap(), sub.Query ?? SelectAllEvents.Instance)
                .OfType<EventOnWire>()
                .Select(ActyxEvent.From(store.NodeId))
                .Buffer(
                    chunkConfig.MaxChunkTime ?? TimeSpan.FromMilliseconds(5),
                    chunkConfig.MaxChunkSize ?? 1000
                )
                .Where(x => x.Count > 0)
                .Select(ActyxEvent.OrderByEventKey)
                .Select(BookKeepingOnChunk(sub.LowerBound));


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

        private static Func<IList<ActyxEvent>, EventChunk> BookKeepingOnChunk(OffsetMap initialLowerBound)
        {
            var lowerBound = initialLowerBound == null ? new OffsetMap() : new OffsetMap(initialLowerBound);
            return events =>
            {
                var upperBound = new OffsetMap(lowerBound);
                events.ToList().ForEach(x => upperBound[x.Meta.Stream] = x.Meta.Offset);
                var chunk = new EventChunk(new OffsetMap(lowerBound), upperBound, events);
                lowerBound = new OffsetMap(upperBound);

                return chunk;
            };
        }

        private static Func<IList<ActyxEvent>, EventChunk> ReverseBookKeepingOnChunk(OffsetMap initialUpperBound)
        {
            var upperBound = initialUpperBound == null ? new OffsetMap() : new OffsetMap(initialUpperBound);
            return events =>
            {
                var lowerBound = new OffsetMap(upperBound);
                var sourcesInChunk = new HashSet<string>();
                events.ToList().ForEach(x =>
                {
                    lowerBound[x.Meta.Stream] = x.Meta.Offset;
                    sourcesInChunk.Add(x.Meta.Stream);
                });
                sourcesInChunk.ToList().ForEach(x =>
                {
                    var bound = lowerBound[x];
                    if (bound == 0)
                    {
                        lowerBound.Remove(x);
                    }
                    else
                    {
                        lowerBound[x] = bound - 1;
                    }
                });

                var chunk = new EventChunk(lowerBound, new OffsetMap(upperBound), events);
                upperBound = new OffsetMap(lowerBound);

                return chunk;
            };
        }
    }
}
