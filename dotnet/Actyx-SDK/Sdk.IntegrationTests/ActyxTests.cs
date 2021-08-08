using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Actyx;
using Sdk.IntegrationTests.Helpers;
using FluentAssertions;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class ActyxTests
    {
        public static IEnumerable<object[]> Clients()
        {
            foreach (var transport in new Transport?[] {
                null,
                Transport.Http,
            })
            {
                var opts = transport is not null ? new ActyxOpts() { Transport = (Transport)transport } : null;
                yield return new object[] { Actyx.Actyx.Create(Constants.TrialManifest, opts) };
            }
        }

        [Theory]
        [MemberData(nameof(Clients))]
        public async void QueryAllKnownAutoCapped(Task<Actyx.Actyx> clientTask)
        {
            var client = await clientTask;
            var meta = await client.Publish(new EventDraft { Tags = new string[] { "Hello", "World" }, Payload = "Hello world" });
            var known = await client.QueryAllKnown(new AutoCappedQuery());

            known.UpperBound[meta.Stream].Should().BeGreaterOrEqualTo(meta.Offset);
            known.Events.Count.Should().BeGreaterOrEqualTo(1);
            known.Events.Select(x => x.Payload.ToString()).Should().Contain("Hello world");
        }
    }
}
