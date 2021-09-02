using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.Utils;
using Sdk.IntegrationTests.Helpers;
using FluentAssertions;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class ActyxTests
    {
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
        public async void ObserveEarliest(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            Tag<string> tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            await client.Publish(tag.Apply("bar"));

            var values = client.ObserveLatest<string>(new () { Query = tag }).ToAsyncEnumerable().GetAsyncEnumerator();

            await values.MoveNextAsync();
            values.Current.Should().Equals("foo");

            await client.Publish(tag.Apply("live0"));
            await values.MoveNextAsync();
            values.Current.Should().Equals("foo");

            await client.Publish(tag.Apply("live1"));
            await values.MoveNextAsync();
            values.Current.Should().Equals("foo");

            await values.DisposeAsync();
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void ObserveLatestTyped(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            Tag<string> tag = new Tag<string>(AxRandom.String(16));

            await client.Publish(tag.Apply("foo"));
            await client.Publish(tag.Apply("bar"));

            var values = client.ObserveLatest<string>(new () { Query = tag }).ToAsyncEnumerable().GetAsyncEnumerator();

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


        [Theory]
        [MemberData(nameof(Opts))]
        public async void ObserveLatestErrorHandling(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            string rid = AxRandom.String(16);
            // Write conflicting types to the same tag...
            Tag<string> tagS = new Tag<string>(rid);
            Tag<int> tagN = new Tag<int>(rid);

            await client.Publish(tagS.Apply("foo"));

            var values = client.ObserveLatest<int>(new () { Query = tagN }).ToAsyncEnumerable().GetAsyncEnumerator();

            Func<Task> act = async () => await values.MoveNextAsync();
            await act.Should().ThrowAsync<System.FormatException>();

            await values.DisposeAsync();
        }


        [Theory]
        [MemberData(nameof(Opts))]
        public async void ObserveBestMatch(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            Tag<int> tag = new Tag<int>(AxRandom.String(16));

            await client.Publish(tag.Apply(50));
            await client.Publish(tag.Apply(60));

            var values = client.ObserveBestMatch<int>(tag, (x, y) => Math.Abs(100 - x.Payload) < Math.Abs(100 - y.Payload)).ToAsyncEnumerable().GetAsyncEnumerator();

            await values.MoveNextAsync();
            values.Current.Should().Equals(60);

            await client.Publish(tag.Apply(120));
            await values.MoveNextAsync();
            values.Current.Should().Equals(120);

            await client.Publish(tag.Apply(95));
            await values.MoveNextAsync();
            values.Current.Should().Equals(95);

            await client.Publish(tag.Apply(110));
            await Task.Delay(5);
            values.Current.Should().Equals(95);

            await values.DisposeAsync();
        }

        [Theory]
        [MemberData(nameof(Opts))]
        public async void ObserveUnorderedReduce(ActyxOpts opts)
        {
            var client = await Actyx.Actyx.Create(Constants.TrialManifest, opts);

            Tag<int> tag = new Tag<int>(AxRandom.String(16));

            await client.Publish(tag.Apply(50));
            await client.Publish(tag.Apply(60));

            var values = client.ObserveUnorderedReduce<int, int>(tag, (x, y) => x + y.Payload, 1).ToAsyncEnumerable().GetAsyncEnumerator();

            await values.MoveNextAsync();
            values.Current.Should().Equals(111);

            await client.Publish(tag.Apply(9));
            await values.MoveNextAsync();
            values.Current.Should().Equals(120);

            await client.Publish(tag.Apply(80));
            await values.MoveNextAsync();
            values.Current.Should().Equals(200);

            await values.DisposeAsync();
        }
    }
}
