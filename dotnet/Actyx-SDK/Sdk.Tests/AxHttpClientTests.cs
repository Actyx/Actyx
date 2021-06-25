using System;
using System.Net.Http;
using Actyx.Sdk.AxHttpClient;
using FluentAssertions;
using Sdk.Tests.Helpers;
using Xunit;

namespace Sdk.Tests
{
    public class AxHttpClientTests
    {
        [Theory]
        [InlineData("")]
        [InlineData("xxx")]
        public async void It_Should_Throw_When_Relative(string uri)
        {
            var ex = await Assert.ThrowsAsync<ArgumentException>(async () => await AxHttpClient.Create(uri, Constants.TrialManifest));
            Assert.Equal($"Base url needs to be an absolute, i.e. 'http://localhost:4454'. Received '{uri}'.", ex.Message);
        }

        [Theory]
        [InlineData("localhost:4454")]
        [InlineData("https://localhost:4454")]
        [InlineData("file://localhost")]
        public async void It_Should_Throw_On_Invalid_Scheme(string uri)
        {
            var ex = await Assert.ThrowsAsync<ArgumentException>(async () => await AxHttpClient.Create(uri, Constants.TrialManifest));
            Assert.Equal($"Only http scheme allowed, i.e. 'http://localhost:4454'. Received '{uri}'.", ex.Message);
        }

        [Fact]
        public async void It_Should_Fail_When_Actyx_Is_Not_Listening_At_Location()
        {
            var uri = "http://localhost:6666";
            var ex = await Assert.ThrowsAsync<HttpRequestException>(async () => await AxHttpClient.Create(uri, Constants.TrialManifest));
            Assert.Equal($"Connection refused", ex.Message);
        }

        [Fact]
        public async void It_Should_Get_App_Id()
        {
            var client = await AxHttpClient.Create(Constants.ApiOrigin, Constants.TrialManifest);
            client.AppId.Should().Equals(Constants.TrialManifest.AppId);
        }

        [Fact]
        public async void It_Should_Get_Node_Id()
        {
            var client = await AxHttpClient.Create(Constants.ApiOrigin, Constants.TrialManifest);
            client.NodeId.Should().NotBeNullOrWhiteSpace();
        }
    }
}
