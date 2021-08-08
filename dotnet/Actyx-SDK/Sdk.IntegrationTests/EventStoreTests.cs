using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx;
using FluentAssertions;
using Sdk.IntegrationTests.Helpers;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class EventStoreTests
    {
        public static IEnumerable<object[]> Clients()
        {
            foreach (var transport in new Transport[] { Transport.Http, Transport.WebSocket })
            {
                yield return new object[] { EventStore.Create(Constants.TrialManifest, new ActyxOpts() { Transport = transport }) };
            }
        }

        [Theory]
        [MemberData(nameof(Clients))]
        public async void It_Should_Get_Offset(Task<IEventStore> clientTask)
        {
            var client = await clientTask;
            var result = await client.Offsets();
            result.Present.Should().NotBeEmpty();
        }

        [Theory]
        [MemberData(nameof(Clients))]
        public async void It_Should_Publish_Events_Then_Query_And_Subscribe(Task<IEventStore> clientTask)
        {
            var client = await clientTask;

            // Publish some events
            var events = Enumerable.Range(1, 10).Select(x => new TestEvent(x)).ToList();
            var results = (await client.Publish(events)).Data;
            results.Should().HaveCount(events.Count);
            var first = results.First();
            first.Lamport.Should().BePositive();
            first.Offset.Should().BeGreaterOrEqualTo(0);
            first.Timestamp.Should().BePositive();

            // Query events
            var query = new TestEventSelection($"FROM {string.Join(" & ", Constants.Tags.Select(x => $"'{x}'"))} SELECT _ / 1");
            var queryResults = await client.Query(null, null, query, EventsOrder.Asc).ToList();
            queryResults.Should().HaveCountGreaterThan(1);
            var offsets = queryResults.Last() as OffsetsOnWire;

            // Query with empty upper bound
            var emptyQueryResult = await client.Query(null, new OffsetMap(), query, EventsOrder.Asc).ToList();
            emptyQueryResult.Should().HaveCount(1);
            (emptyQueryResult.First() as OffsetsOnWire).Offsets.Should().BeEmpty();

            // Subscribe to events
            var subscribeResults = await client.Subscribe(null, query).Take(2).ToList();
            subscribeResults.Should().HaveCount(2);
        }

        public static IEnumerable<object[]> EmptyEvents()
        {
            foreach (var client in Clients().SelectMany(c => c.Cast<Task<IEventStore>>()))
            {
                yield return new object[] { client, null };
                yield return new object[] { client, new TestEvent[0] };
            }
        }

        [Theory()]
        [MemberData(nameof(EmptyEvents))]
        public async void It_Should_Complete_When_Nothing_To_Publish(Task<IEventStore> clientTask, IEnumerable<IEventDraft> emptyEvents)
        {
            var client = await clientTask;
            var result = await client.Publish(emptyEvents);
            result.Data.Should().HaveCount(0);
        }
    }
}
