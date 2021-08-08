using System.Collections.Generic;
using System.IO;
using Actyx;
using DeepEqual.Syntax;
using Newtonsoft.Json;
using Xunit;

namespace Sdk.Tests
{
    public class EventStoreFormatsTests
    {
        private readonly JsonSerializer serializer = EventStoreSerializer.Create(pretty: false);

        public static IEnumerable<object[]> Payloads() =>
            new List<object[]>()
            {
                new object[] {
                    new { LowerBound = new OffsetMap(), UpperBound = new OffsetMap(), Query = "FROM allEvents", Order = EventsOrder.Asc },
                    @"{""lowerBound"":{},""upperBound"":{},""query"":""FROM allEvents"",""order"":""asc""}",
                },
                // TODO add more
            };

        [Theory()]
        [MemberData(nameof(Payloads))]
        public void It_Should_Roundtrip<T>(T value, string jsonStr)
        {
            var writer = new StringWriter();
            serializer.Serialize(writer, value);
            string json = writer.ToString();
            Assert.Equal(json, jsonStr);
            var deserialized = serializer.Deserialize<T>(new JsonTextReader(new StringReader(json)));
            deserialized.ShouldDeepEqual(value);
        }
    }
}
