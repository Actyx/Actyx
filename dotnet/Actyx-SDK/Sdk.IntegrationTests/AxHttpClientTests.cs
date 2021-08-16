using System;
using System.Net.Http;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Utils;
using Sdk.IntegrationTests.Helpers;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class AxHttpClientTests
    {
        private readonly JsonContentConverter converter = new(DefaultJsonSerializer.Create());

        private AxHttpClient Create(string uri)
        {
            var apiUri = new Uri(new Uri(uri), "api/v2/");
            return new AuthenticatedClient(Constants.TrialManifest, new Uri(apiUri, "events/"), new Uri(apiUri, "auth"), converter);
        }

        [Theory]
        [InlineData("")]
        [InlineData("xxx")]
        public void It_Should_Throw_For_Invalid_Uris(string uri)
        {
            Assert.Throws<UriFormatException>(() => Create(uri));
        }

        [Theory]
        [InlineData("localhost:4454")]
        [InlineData("https://localhost:4454")]
        [InlineData("file://localhost")]
        public void It_Should_Throw_On_Invalid_Scheme(string uri)
        {
            var ex = Assert.Throws<ArgumentException>(() => Create(uri));
            Assert.Equal($"Only http scheme allowed. Received '{new Uri(uri).Scheme}'.", ex.Message);
        }

        [Fact]
        public void It_Should_Fail_When_Actyx_Is_Not_Listening_At_Location()
        {
            var uri = "http://localhost:6666";
            var ex = Assert.ThrowsAsync<HttpRequestException>(async () => await Create(uri).Get<object>(""));
        }
    }
}
