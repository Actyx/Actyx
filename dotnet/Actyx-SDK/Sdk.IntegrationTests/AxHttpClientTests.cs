using System;
using System.Net.Http;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Utils;
using FluentAssertions;
using Sdk.IntegrationTests.Helpers;
using Xunit;

namespace Sdk.IntegrationTests
{
    public class AxHttpClientTests
    {
        private readonly JsonContentConverter converter = new(DefaultJsonSerializer.Create());

        private async Task<AxHttpClient> Create(string uri) =>
            await AxHttpClient.Create(uri, Constants.TrialManifest, converter);

        [Theory]
        [InlineData("")]
        [InlineData("xxx")]
        public async void It_Should_Throw_When_Relative(string uri)
        {
            var ex = await Assert.ThrowsAsync<ArgumentException>(async () => await Create(uri));
            Assert.Equal($"Base url needs to be an absolute, i.e. 'http://localhost:4454'. Received '{uri}'.", ex.Message);
        }

        [Theory]
        [InlineData("localhost:4454")]
        [InlineData("https://localhost:4454")]
        [InlineData("file://localhost")]
        public async void It_Should_Throw_On_Invalid_Scheme(string uri)
        {
            var ex = await Assert.ThrowsAsync<ArgumentException>(async () => await Create(uri));
            Assert.Equal($"Only http scheme allowed, i.e. 'http://localhost:4454'. Received '{uri}'.", ex.Message);
        }

        [Fact]
        public async void It_Should_Fail_When_Actyx_Is_Not_Listening_At_Location()
        {
            var uri = "http://localhost:6666";
            var ex = await Assert.ThrowsAsync<HttpRequestException>(async () => await Create(uri));
        }

        [Fact()]
        public async void It_Should_Get_App_Id()
        {
            var opts = new ActyxOpts();
            string uri = $"http://{opts.Host}:{opts.Port}/api/v2/";
            var client = await Create(uri);
            client.AppId.Should().Equals(Constants.TrialManifest.AppId);
        }

        [Fact()]
        public async void It_Should_Get_Node_Id()
        {
            var opts = new ActyxOpts();
            string uri = $"http://{opts.Host}:{opts.Port}/api/v2/";
            var client = await Create(uri);
            client.NodeId.ToString().Should().NotBeNullOrWhiteSpace();
        }
    }
}
