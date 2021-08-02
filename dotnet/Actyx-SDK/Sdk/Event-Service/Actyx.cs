using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;

namespace Actyx
{
    public enum Transport
    {
        Http,
        WebSocket,
    }

    public class ActyxOpts
    {
        private static uint? PortNumber(string s) => s is null ? (uint?)null : Convert.ToUInt32(s);

        public ActyxOpts()
        {
            Transport = Transport.Http;
            Host = Environment.GetEnvironmentVariable("AX_CLIENT_HOST") ?? "localhost";
            Port = PortNumber(Environment.GetEnvironmentVariable("AX_CLIENT_API_PORT")) ?? 4454;
        }

        /** Host of the Actxy service. This defaults to localhost and should stay localhost in almost all cases. */
        public string Host { get; set; }

        /** API port of the Actyx service. Defaults to 4454. */
        public uint Port { get; set; }

        /** Whether to use plain http or websocket to communicate with the Actyx service. Defaults
         * to WebSocket. */
        public Transport Transport { get; set; }

        // Implement me.
        // public Action OnConnectionLost { get; set; }
    }

    public class Actyx : IEventFns, IDisposable
    {
        public static async Task<Actyx> Create(AppManifest manifest)
        {
            return await Create(manifest, null);
        }

        public static async Task<Actyx> Create(AppManifest manifest, ActyxOpts options) =>
            new Actyx(await EventStore.Create(manifest, options));

        private readonly IEventStore store;
        private Actyx(IEventStore store)
        {
            this.store = store;
        }

        public void Dispose()
        {
            store.Dispose();
        }

        public NodeId NodeId
        {
            get => store.NodeId;
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
