using System.Collections.Generic;
using Actyx;
using Xunit;
using Newtonsoft.Json.Linq;
using DeepEqual.Syntax;

namespace Sdk.Tests
{
    public class ProtocolTests
    {
        static void Roundtrip<T>(T value)
        {
            var serialized = Proto<T>.Serialize(value);
            var deserialized = Proto<T>.Deserialize(serialized);
            deserialized.ShouldDeepEqual(value);
        }

        [Fact]
        public void Incoming()
        {
            new List<IIncoming> {
                new Next { RequestId = 1, Payload = new JObject[] { JObject.Parse(@"{ ""this is"": ""the payload"" }") } },
                new Complete { RequestId = 1 },
                new Error
                {
                    RequestId = 1,
                    Kind = new UnknownEndpoint
                    {
                        Endpoint = "invalid",
                        ValidEndpoints = new string[] { "valid1", "valid2" },
                    }
                },
                new Error { RequestId = 1, Kind = new InternalError { } },
                new Error { RequestId = 1, Kind = new BadRequest { Message = "It's really bad!" } },
                new Error
                {
                    RequestId = 1,
                    Kind = new ServiceError
                    {
                        Value = JObject.Parse(@"{ ""some"": ""nested"", ""props"": ""right here"" }")
                    }
                }
            }.ForEach(Roundtrip);
        }

        [Fact]
        public void Outgoing()
        {
            new List<IOutgoing> {
                new Request { RequestId = 1, ServiceId = "some_service", Payload = JObject.Parse(@"{ ""this is"": ""the payload"" }") },
                new Cancel { RequestId = 1 },
            }.ForEach(Roundtrip);
        }
    }
}
