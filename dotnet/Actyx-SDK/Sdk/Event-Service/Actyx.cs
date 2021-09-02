using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Newtonsoft.Json.Linq;
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
        public static async Task<Actyx> Create(AppManifest manifest, ActyxOpts options = null)
        {
            options ??= new ActyxOpts();
            return new Actyx(manifest.AppId, await EventStore.Create(manifest, options));
        }

        private readonly string appId;
        private readonly IEventStore store;

        private Actyx(string appId, IEventStore store)
        {
            this.appId = appId;
            this.store = store;
        }

        public void Dispose()
        {
            store.Dispose();
        }

        public string AppId
        {
            get => this.appId;
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
            var events = wireEvents.OfType<EventOnWire>().Select(MkAxEvt.From(NodeId)).ToList();
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

        public async Task<IList<ActyxEvent<JToken>>> QueryKnownRange(RangeQuery query)
        {
            var wireEvents = await QueryKnown(query);
            var events = wireEvents.OfType<EventOnWire>().Select(MkAxEvt.From(NodeId)).ToList();

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
                .Select(MkAxEvt.From(NodeId))
                .Buffer(chunkSize)
                .Select(query.Order == EventsOrder.Asc ? BookKeepingOnChunk(query.LowerBound) : ReverseBookKeepingOnChunk(query.UpperBound));
        }

        public IObservable<ActyxEvent<JToken>> Subscribe(EventSubscription sub) => store
            .Subscribe(sub.LowerBound ?? new OffsetMap(), sub.Query ?? SelectAllEvents.Instance)
            .OfType<EventOnWire>()
            .Select(MkAxEvt.From(NodeId));

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub) =>
            SubscribeChunked(sub, new ChunkingOptions { MaxChunkSize = 1000, MaxChunkTime = TimeSpan.FromMilliseconds(5) });

        public IObservable<EventChunk> SubscribeChunked(EventSubscription sub, ChunkingOptions chunkConfig) =>
             store
                .Subscribe(sub.LowerBound ?? new OffsetMap(), sub.Query ?? SelectAllEvents.Instance)
                .OfType<EventOnWire>()
                .Select(MkAxEvt.From(NodeId))
                .Buffer(
                    chunkConfig.MaxChunkTime ?? TimeSpan.FromMilliseconds(5),
                    chunkConfig.MaxChunkSize ?? 1000
                )
                .Where(x => x.Count > 0)
                .Select(ActyxEvent<JToken>.OrderByEventKey)
                .Select(BookKeepingOnChunk(sub.LowerBound));


        private async Task<IEnumerable<IResponseMessage>> QueryKnown(RangeQuery query)
        {
            var wireEvents = await store.Query(
                query.LowerBound,
                query.UpperBound,
                query.Query ?? SelectAllEvents.Instance,
                query.Order
            ).ToList();

            return wireEvents;
        }

        private static Func<IList<ActyxEvent<JToken>>, EventChunk> BookKeepingOnChunk(OffsetMap initialLowerBound)
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

        private static Func<IList<ActyxEvent<JToken>>, EventChunk> ReverseBookKeepingOnChunk(OffsetMap initialUpperBound)
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

        private ActyxEventMetadata ConvertMetadata(EventPublishMetadata publishedMetadata, IEnumerable<string> tags)
        {
            return new ActyxEventMetadata(
                publishedMetadata.Timestamp, publishedMetadata.Lamport, publishedMetadata.Offset,
                this.AppId, publishedMetadata.Stream, tags, this.NodeId);
        }

        private IObservable<EventOnWire> ReadOne(
            IEventSelection query,
            OffsetMap present,
            EventsOrder ord
        )
        {
            return store.Query(null, present, query, ord)
                .OfType<EventOnWire>()
                .Take(1)
                .DefaultIfEmpty(null);
        }

        public IObservable<ActyxEvent<E>> ObserveLatest<E>(LatestQuery<E> q) {
            if (q.EventComparison == EventComparison.Timestamp) {
                return store
                    .Subscribe(q.LowerBound, q.Query)
                    .SelectMany(
                        MkEmitIf<E>(
                            null,
                            (candidate, current) => candidate.Timestamp.CompareTo(current.Timestamp) > 0
                    ));
            }

            // We can optimise Lamport-ordering by letting the store figure out the current latest event
            return Observable.FromAsync(this.Present)
                .SelectMany(present => {
                    var firstValue = ReadOne(q.Query, present, EventsOrder.Desc);

                    var values = firstValue.SelectMany(currentLatest => {
                        return store
                            .Subscribe(present, q.Query)
                            .SelectMany(MkEmitIf<E>(currentLatest, (candidate, current) => candidate.CompareTo(current) > 0));
                    });

                    return values;
                });
        }

        public IObservable<ActyxEvent<E>> ObserveEarliest<E>(LatestQuery<E> q) {
            if (q.EventComparison == EventComparison.Timestamp) {
                return store
                    .Subscribe(q.LowerBound, q.Query)
                    .SelectMany(MkEmitIf<E>(null, (candidate, current) => candidate.CompareTo(current) < 0));
            }

            // We can optimise Lamport-ordering by letting the store figure out the current earliest event
            return Observable.FromAsync(this.Present)
                .SelectMany(present => {
                    var firstValue = ReadOne(q.Query, present, EventsOrder.Asc);

                    var values = firstValue.SelectMany(currentLatest => {
                        return store
                            .Subscribe(present, q.Query)
                            .SelectMany(MkEmitIf<E>(currentLatest, (candidate, current) => candidate.CompareTo(current) < 0));
                    });

                    return values;
                });
        }

        private Func<IResponseMessage, ActyxEvent<E>[]> MkEmitIf<E>(EventOnWire initialLatest, Func<EventOnWire, EventOnWire, bool> shouldReplaceCur)
        {
            var deser = MkAxEvt.DeserTyped<E>(NodeId);
            EventOnWire latest = initialLatest;

            ActyxEvent<E>[] empty =  new ActyxEvent<E>[] {};

            bool live = false;

            ActyxEvent<E>[] EmitIfConditionMet(IResponseMessage r) {
                switch (r) {
                    case (OffsetsOnWire):
                    {
                        live = true;
                        if (latest != null) {
                            return new[] { deser(latest) };
                        }

                        break;
                    }
                    case (EventOnWire evt):
                    {
                        if (latest is null || shouldReplaceCur(evt, latest)) {
                            latest = evt;

                            if (live) {
                                return new[] { deser(latest) };
                            }
                        }

                        break;
                    }
                }

                return empty;
            }

            return EmitIfConditionMet;
        }

        public IObservable<ActyxEvent<E>> ObserveBestMatch<E>(
            IFrom<E> query,
            Func<ActyxEvent<E>, ActyxEvent<E>, bool> shouldReplace
        )
        {
            var deser = MkAxEvt.DeserTyped<E>(NodeId);
            ActyxEvent<E> curBestMatch = null;

            ActyxEvent<E>[] empty =  new ActyxEvent<E>[] {};

            bool live = false;

            // We can’t use MkEmitIf because we have to deserialize *before* checking the condition here.
            ActyxEvent<E>[] EmitIfBetterMatch(IResponseMessage r) {
                switch (r)
                {
                    case (OffsetsOnWire):
                    {
                        live = true;
                        if (curBestMatch != null) {
                            return new[] { curBestMatch };
                        }
                        break;
                    }

                    case (EventOnWire evt):
                    {
                        var nextEvent = deser(evt);
                        if (curBestMatch is null || shouldReplace(nextEvent, curBestMatch)) {
                            curBestMatch = nextEvent;

                            if (live) {
                                return new[] { nextEvent };
                            }
                        }
                        break;
                    }
                }

                return empty;
            }

             return store
                .Subscribe(null, query)
                .SelectMany(EmitIfBetterMatch);
        }


        public IObservable<R> ObserveUnorderedReduce<R, E>(
            IFrom<E> query,
            Func<R, ActyxEvent<E>, R> reduce,
            R initial
        )
        {
            var deser = MkAxEvt.DeserTyped<E>(NodeId);

            IObservable<R> valueFeed = Observable.FromAsync(this.Present)
                .SelectMany(present => {
                    IObservable<R> curVal = store
                        .Query(
                            null,
                            present,
                            query,
                            EventsOrder.StreamAsc
                            )
                        .OfType<EventOnWire>()
                        .Select(deser)
                        .Aggregate(initial, reduce);

                    return curVal.SelectMany(v => store
                                             .Subscribe(present, query)
                                             .OfType<EventOnWire>()
                                             .Select(deser)
                                             .Scan(v, reduce)
                                             .StartWith(v));
                });

            return valueFeed;
        }

        public async Task<ActyxEventMetadata> Publish(IEventDraft eventDraft)
        {
            var res = await store.Publish(new[] { eventDraft });
            return ConvertMetadata(res.Data.First(), eventDraft.Tags);
        }
    }
}
