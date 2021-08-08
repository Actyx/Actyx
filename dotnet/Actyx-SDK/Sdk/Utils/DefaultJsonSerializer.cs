using System.Collections.Generic;
using System.Text.RegularExpressions;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Serialization;

namespace Actyx.Sdk.Utils
{
    public static class DefaultJsonSerializer
    {
        public static JsonSerializer Create(List<JsonConverter> converters = null, bool pretty = true)
        {
            // enum values are (de-)serialized in kebab-case
            var enumConverter = new StringEnumConverter() { NamingStrategy = new KebabCaseNamingStrategy() };
            converters ??= new List<JsonConverter>();
            converters.Add(enumConverter);

            return JsonSerializer.Create(new JsonSerializerSettings()
            {
                ContractResolver = new DefaultContractResolver
                {
                    NamingStrategy = new CamelCaseNamingStrategy
                    {
                        ProcessDictionaryKeys = false,
                        OverrideSpecifiedNames = true
                    }
                },
                Formatting = pretty ? Formatting.Indented : Formatting.None,
                DefaultValueHandling = DefaultValueHandling.Include,
                ReferenceLoopHandling = ReferenceLoopHandling.Ignore,
                DateParseHandling = DateParseHandling.None,
                NullValueHandling = NullValueHandling.Ignore,
                Converters = converters,
            });
        }
    }

    class KebabCaseNamingStrategy : NamingStrategy
    {
        protected override string ResolvePropertyName(string name) =>
            Regex.Replace(name, "([a-z](?=[A-Z])|[A-Z](?=[A-Z][a-z]))", "$1-").ToLower();
    }
}
