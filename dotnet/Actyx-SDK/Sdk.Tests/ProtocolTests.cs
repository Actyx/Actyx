using System.Collections.Generic;
using Actyx;
using Xunit;
using FluentAssertions;
using Newtonsoft.Json.Linq;

namespace Sdk.Tests
{
    public class ProtocolTests
    {
        [Fact]
        public void DeserializeIncoming()
        {
            var expected = new List<Incoming> {
                new Next     { RequestId = 1, Payload = JObject.Parse(@"{ ""this is"": ""the payload""}")},
                new Complete { RequestId = 1},
                new Error    { RequestId = 1, Kind = new UnknownEndpoint{
                    Endpoint = "invalid",
                    ValidEndpoints = new string[]{ "valid1", "valid2" },
                }},
                new Error { RequestId = 1, Kind = new InternalError{}},
                new Error { RequestId = 1, Kind = new BadRequest{ Message = "It's really bad!"}},
                new Error { RequestId = 1, Kind = new ServiceError{
                    Value = JObject.Parse(@"{ ""some"": ""nested"", ""props"": ""right here"" }")
                }},
            };
            var input = new List<string> {
                @"{ ""type"": ""next"", ""requestId"": 1, ""payload"": { ""this is"": ""the payload""} }",
                @"{ ""type"": ""complete"", ""requestId"": 1 }",
                @"{ ""type"": ""error"", ""requestId"": 1, ""kind"": {
                    ""type"": ""unknownEndpoint"",
                    ""endpoint"": ""invalid"",
                    ""validEndpoints"": [""valid1"", ""valid2""]
                }}",
                @"{ ""type"": ""error"", ""requestId"": 1, ""kind"": {
                    ""type"": ""internalError""
                }}",
                @"{ ""type"": ""error"", ""requestId"": 1, ""kind"": {
                    ""type"": ""badRequest"",
                    ""message"": ""It's really bad!""
                }}",
                @"{ ""type"": ""error"", ""requestId"": 1, ""kind"": {
                    ""type"": ""serviceError"",
                    ""value"": { ""some"": ""nested"", ""props"": ""right here"" }
                }}",
            };
            input.ConvertAll(Incoming.Deserialize).Should().Equals(expected);
            expected.ConvertAll(Incoming.Serialize).Should().Equals(input);
        }

        [Fact]
        public void DeserializeOutgoing()
        {
            var expected = new List<Outgoing> {
                new Request { RequestId = 1, ServiceId = "some_service", Payload = JObject.Parse(@"{ ""this is"": ""the payload""}")},
                new Cancel  { RequestId = 1},
            };
            var input = new List<string> {
                @"{ ""type"": ""request"", ""requestId"": 1, ""serviceId"": ""some_service"", ""payload"": { ""this is"": ""the payload""} }",
                @"{ ""type"": ""cancel"", ""requestId"": 1 }",
            };
            input.ConvertAll(Outgoing.Deserialize).Should().Equals(expected);
            expected.ConvertAll(Outgoing.Serialize).Should().Equals(input);
        }
    }
}
