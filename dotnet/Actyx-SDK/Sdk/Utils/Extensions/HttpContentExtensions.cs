using System.Net.Http;
using System.Threading.Tasks;
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
        };

        public static async Task<T> ReadFromJsonAsync<T>(this HttpContent content)
        {
            var data = await content.ReadAsStringAsync();
            return JsonConvert.DeserializeObject<T>(data, JsonSettings);
        }
    }
}
