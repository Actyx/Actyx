using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Utils;
using FluentAssertions;
using Xunit;

namespace Sdk.Tests
{
    public class JsonContentConverterTests
    {
        private readonly JsonContentConverter converter = new(DefaultJsonSerializer.Create(pretty: false));

        class X
        {
            public string A;
            public int B;
        }

        [Fact]
        public async void It_Should_Serialize()
        {
            var x = new X() { A = "a", B = 2 };
            var json = @"{""a"":""a"",""b"":2}";
            var content = converter.ToContent(x);
            (await content.ReadAsStringAsync()).Should().BeEquivalentTo(json);
            (await converter.FromContent<X>(content)).Should().BeEquivalentTo(x);

        }
    }
}
