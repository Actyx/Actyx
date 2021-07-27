using System.Collections.Generic;
using System.Net.Http;
using System.Threading.Tasks;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Serialization;

namespace Actyx.Sdk.Utils.Extensions
{
    public static class HttpContentExtensions
    {
        public static JsonSerializerSettings JsonSettings = new()
        {
            ContractResolver = new DefaultContractResolver
            {
                NamingStrategy = new CamelCaseNamingStrategy
                {
                    ProcessDictionaryKeys = false,
                    OverrideSpecifiedNames = true
                }
            },
            Formatting = Formatting.Indented,
            DefaultValueHandling = DefaultValueHandling.Include,
            ReferenceLoopHandling = ReferenceLoopHandling.Ignore,
            DateParseHandling = DateParseHandling.None,
            NullValueHandling = NullValueHandling.Ignore,
            Converters = new List<JsonConverter>
            {
                JsonSubtypesConverterBuilder
                    .Of(typeof(IEventOnWire), "type")
                    .RegisterSubtype<OffsetsOnWire>("offsets")
                    .RegisterSubtype<EventOnWire>("event")
                    .SerializeDiscriminatorProperty() // can't be set with Attributes
                    .Build(),
                JsonSubtypesConverterBuilder
                    .Of(typeof(ISubscribeMonotonicResponse), "type")
                    .RegisterSubtype<SubscribeMonotonicEventResponse>("event")
                    .RegisterSubtype<SubscribeMonotonicOffsetsResponse>("offsets")
                    .RegisterSubtype<SubscribeMonotonicTimeTravelResponse>("timeTravel")
                    .SerializeDiscriminatorProperty() // can't be set with Attributes
                    .Build(),
            }
        };

        public static async Task<T> ReadFromJsonAsync<T>(this HttpContent content)
        {
            var data = await content.ReadAsStringAsync();
            return JsonConvert.DeserializeObject<T>(data, JsonSettings);
        }
    }
}
