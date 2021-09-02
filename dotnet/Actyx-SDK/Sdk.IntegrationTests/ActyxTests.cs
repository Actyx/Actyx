using System.Collections.Generic;
using System.Linq;
using Actyx;
using Actyx.Sdk.Utils;
using Sdk.IntegrationTests.Helpers;
using FluentAssertions;
using Xunit;
using System.Threading.Tasks;
using System.Reactive.Linq;
using Xunit.Abstractions;
using Actyx.Sdk.Formats;
using System;

namespace Sdk.IntegrationTests
{
    public class ActyxTests
    {

        private readonly ITestOutputHelper output;

        public ActyxTests(ITestOutputHelper output)
        {
            this.output = output;
        }

        public static IEnumerable<object[]> Opts()
        {
            foreach (var transport in new Transport?[] {
                null,
                Transport.Http,
                Transport.WebSocket,
            })
            {
                yield return new object[] { transport is not null ? new ActyxOpts() { Transport = (Transport)transport } : null };
            }
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void QueryAllKnownAutoCapped(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);
            var meta = await client.Publish(new EventDraft { Tags = new string[] { "Hello", "World" }, Payload = "Hello world" });
            var known = await client.QueryAllKnown(new AutoCappedQuery());

            known.UpperBound[meta.Stream].Should().BeGreaterOrEqualTo(meta.Offset);
            known.Events.Count.Should().BeGreaterOrEqualTo(1);
            known.Events.Select(x => x.Payload.ToString()).Should().Contain("Hello world");
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void Subscribe(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);
            var tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            var _ = Task.Run(async () =>
            {
                await client.Publish(tag.Apply("bar"));
            });

            var res = await client
                .Subscribe(new EventSubscription() { Query = new Aql(tag.ToAql()), })
                .Take(2)
                .ToList();
            res.Should().HaveCount(2);
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void SubscribeChunked(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);
            var tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            var _ = Task.Run(async () =>
            {
                await client.Publish(tag.Apply("bar"));
            });

            var res = await client
                .SubscribeChunked(
                    new EventSubscription() { Query = new Aql(tag.ToAql()), },
                    new ChunkingOptions() { MaxChunkTime = TimeSpan.FromMilliseconds(1000) }
                )
                .Take(1)
                .ToList();
            res.Should().HaveCount(1);
            res[0].Events.Should().HaveCount(2);
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void SubscribeMonotonic(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);
            var tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            var _ = Task.Run(async () =>
            {
                // TODO simulate time travel
                await client.Publish(tag.Apply("bar"));
            });

            var sessionId = AxRandom.String(16);
            var res = await client
                .SubscribeMonotonic(new EventSubscription() { Query = new Aql(tag.ToAql()), }, sessionId)
                .Do(x => output.WriteLine($">>> x"))
                .Take(2)
                .ToList();
            res.Should().HaveCount(2);
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void SubscribeMonotonicChunked(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);
            var tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            var _ = Task.Run(async () =>
            {
                // TODO simulate time travel
                await client.Publish(tag.Apply("bar"));
            });

            var sessionId = AxRandom.String(16);
            var res = await client
                .SubscribeMonotonicChunked(
                    new EventSubscription() { Query = new Aql(tag.ToAql()), },
                    sessionId,
                    new ChunkingOptions() { MaxChunkTime = TimeSpan.FromMilliseconds(1000) }
                )
                .Take(1)
                .ToList();
            res.Should().HaveCount(1);
            res[0].Events.Should().HaveCount(2);
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void ObserveLatestTyped(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            var tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            await client.Publish(tag.Apply("bar"));

            var values = client.ObserveLatest<string>(new() { Query = tag }).ToAsyncEnumerable().GetAsyncEnumerator();

            await values.MoveNextAsync();
            values.Current.Should().Equals("bar");

            await client.Publish(tag.Apply("live0"));
            await values.MoveNextAsync();
            values.Current.Should().Equals("live0");

            await client.Publish(tag.Apply("live1"));
            await values.MoveNextAsync();
            values.Current.Should().Equals("live1");

            await values.DisposeAsync();
        }
    }
}
