using Actyx;
using Actyx.Sdk.Formats;
using FluentAssertions;
using Xunit;

namespace Sdk.Tests
{
    public class ActyxEventMetadataTests
    {
        [Theory]
        [InlineData(0, "00000000000/xxxxx-0")]
        [InlineData(7, "00000000007/xxxxx-0")]
        [InlineData(666, "00000000666/xxxxx-0")]
        [InlineData(4_294_967_295, "4294967295/xxxxx-0")]
        public void It_Should_Mk_EventId(uint lamport, string expected)
        {
            var ev = new EventOnWire
            {
                Lamport = lamport,
                Stream = "xxxxx-0"
            };
            var result = new ActyxEventMetadata(ev, new NodeId("--node-id--"));
            result.EventId.Should().Equals(expected);
        }
    }
}
