using FluentAssertions;
using Xunit;

namespace Sdk.Tests
{
    public class AdderTets
    {
        [Fact]
        public void Adder_Should_Add_Numbers()
        {
            (1 + 3).Should().Equals(4);
        }
    }
}
