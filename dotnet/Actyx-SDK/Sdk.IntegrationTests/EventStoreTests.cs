using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.Utils;
using FluentAssertions;
using Sdk.IntegrationTests.Helpers;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class EventStoreTests
    {
        public static IEnumerable<object[]> Transports()
        {
            yield return new object[] { Transport.Http };
            yield return new object[] { Transport.WebSocket };
        }

        private static async Task<IEventStore> MkStore(Transport transport) =>
            await EventStore.Create(Constants.TrialManifest, new ActyxOpts() { Transport = transport });

        [Theory]
        [MemberData(nameof(Transports))]
        public async void It_Should_Get_Offsets(Transport transport)
        {
            using var store = await MkStore(transport);
            var result = await store.Offsets();
            result.Present.Should().NotBeEmpty();
        }

        [Theory]
        [MemberData(nameof(Transports))]
        public async void It_Should_Get_App_Id(Transport transport)
        {
            using var store = await MkStore(transport);
            store.AppId.Should().Equals(Constants.TrialManifest.AppId);
        }

        [Theory]
        [MemberData(nameof(Transports))]
        public async void It_Should_Get_Node_Id(Transport transport)
        {
            using var store = await MkStore(transport);
            store.NodeId.ToString().Should().NotBeNullOrWhiteSpace();
        }

        [Theory]
        [MemberData(nameof(Transports))]
        public async void It_Should_Publish_Events_Then_Query_And_Subscribe(Transport transport)
        {
            using var store = await EventStore.Create(Constants.TrialManifest, new ActyxOpts() { Transport = transport });
            var tags = new List<string>() { "42", "order", "dotnet", AxRandom.String(8) };
            var numEvents = 10;

            // Publish some events
            var events = Enumerable.Range(0, numEvents).Select(x => new TestEvent(x, tags)).ToList();
            var results = (await store.Publish(events)).Data;
            results.Should().HaveCount(events.Count);
            var first = results.First();
            first.Lamport.Should().BePositive();
            first.Offset.Should().BeGreaterOrEqualTo(0);
            first.Timestamp.Should().BePositive();

            // Query events
            var query = new TestEventSelection($"FROM {string.Join(" & ", tags.Select(x => $"'{x}'"))} SELECT _ / 1");
            var queryResults = await store.Query(null, null, query, EventsOrder.Asc).ToList();
            queryResults.Should().HaveCountGreaterThan(1);
            var offsets = queryResults.Last() as OffsetsOnWire;

            // Query with empty upper bound
            var emptyQueryResult = await store.Query(null, new OffsetMap(), query, EventsOrder.Asc).ToList();
            emptyQueryResult.Should().HaveCount(1);
            (emptyQueryResult.First() as OffsetsOnWire).Offsets.Should().BeEmpty();

            // Subscribe to events
            var subscribeResults = await store.Subscribe(null, query).Take(2).ToList();
            subscribeResults.Should().HaveCount(2);
        }

        public static IEnumerable<object[]> EmptyEvents()
        {
            foreach (var transport in Transports().SelectMany(c => c.Cast<Transport>()))
            {
                yield return new object[] { transport, null };
                yield return new object[] { transport, System.Array.Empty<TestEvent>() };
            }
        }

        [Theory()]
        [MemberData(nameof(EmptyEvents))]
        public async void It_Should_Complete_When_Nothing_To_Publish(Transport transport, IEnumerable<IEventDraft> emptyEvents)
        {
            using var store = await MkStore(transport);
            var result = await store.Publish(emptyEvents);
            result.Data.Should().HaveCount(0);
        }
    }
}
