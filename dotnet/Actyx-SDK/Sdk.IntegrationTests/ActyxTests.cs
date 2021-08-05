using System.Collections.Generic;
using System.Threading.Tasks;
using Actyx;
using Sdk.IntegrationTests.Helpers;
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
            var _ = await client.QueryAllKnown(new AutoCappedQuery());
        }
    }
}
