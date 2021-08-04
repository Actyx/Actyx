using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Utils;
using Xunit;

namespace Sdk.Tests
{
    public class JsonContentTests
    {
        [Fact]
        public async void It_Should_Serialize()
        {
            var json = new JsonContent(new { A = "a", B = 2 }, DefaultJsonSerializer.Create(pretty: false));
            Assert.Equal(@"{""a"":""a"",""b"":2}", await json.ReadAsStringAsync());
        }
    }
}
