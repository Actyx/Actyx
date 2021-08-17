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
    }
}
