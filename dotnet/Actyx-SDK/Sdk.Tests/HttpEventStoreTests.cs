using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using FluentAssertions;
using Sdk.Tests.Helpers;
using Xunit;

namespace Sdk.Tests
{
    public class HttpEventStoreTests : IAsyncLifetime
    {

        private AxHttpClient client;
        private HttpEventStore es;
        public async Task InitializeAsync()
        {
            client = await AxHttpClient.Create(Constants.ApiOrigin, Constants.TrialManifest);
            es = new HttpEventStore(client);
        }

        public Task DisposeAsync() => Task.CompletedTask;

        [Fact]
        public async void It_Should_Get_Offset()
        {
            var result = await es.Offsets();
            // stream 1 is for discovery events, which is the only stream guaranteed to have events from the start
            // (there are at least two addresses bound: primary interface and localhost, so at least two events)
            var key = $"{client.NodeId}-1";
            result.Present[key].Should().BeGreaterThan(0);
        }

        [Fact]
        public async void It_Should_Publish_Events_Then_Query_And_Subscribe()
        {
            // Publish some events
            var events = Enumerable.Range(1, 10).Select(x => new TestEvent($"event {x}")).ToList();
            var results = (await es.Publish(events)).Data;
            results.Should().HaveCount(events.Count);
            var first = results.First();
            first.Lamport.Should().BePositive();
            first.Offset.Should().BeGreaterOrEqualTo(0);
            first.Timestamp.Should().BePositive();
            first.Stream.Should().Equals($"{client.NodeId}-0");

            // Query events
            var query = new TestEventSelection($"FROM {string.Join(" & ", Constants.Tags.Select(x => $"'{x}'"))}");
            var queryResults = await es.Query(null, null, query, EventsOrder.Asc).ToList();
            queryResults.Should().HaveCountGreaterThan(1);
            var offsets = queryResults.Last() as OffsetsOnWire;
            var eventsStreamKey = $"{client.NodeId}-0";
            offsets.Offsets[eventsStreamKey].Should().BePositive();

            // Query with empty upper bound
            var emptyQueryResult = await es.Query(null, new OffsetMap(), query, EventsOrder.Asc).ToList();
            emptyQueryResult.Should().HaveCount(1);
            (emptyQueryResult.First() as OffsetsOnWire).Offsets.Should().BeEmpty();

            // Subscribe to events
            var lowerBound = new OffsetMap { { $"{client.NodeId}-0", 1 }, };
            var subscribeResults = await es.Subscribe(lowerBound, query).Take(2).ToList();
            subscribeResults.Should().HaveCount(2);
        }

        public static IEnumerable<object[]> It_Should_Complete_When_Nothing_To_Publish_TestData()
        {
            yield return new object[] { null };
            yield return new object[] { new TestEvent[0] };
        }

        [Theory]
        [MemberData(nameof(It_Should_Complete_When_Nothing_To_Publish_TestData))]
        public async void It_Should_Complete_When_Nothing_To_Publish(IEnumerable<IEventDraft> events)
        {
            var result = await es.Publish(events);
            result.Data.Should().HaveCount(0);
        }
    }
}
