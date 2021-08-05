using System.IO;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Threading.Tasks;
using Actyx.Sdk.Utils;
using Newtonsoft.Json;

namespace Actyx.Sdk.AxHttpClient
{
    public class JsonContentConverter
    {
        private readonly JsonProtocol protocol;
        public JsonContentConverter() : this(new JsonProtocol(DefaultJsonSerializer.Create())) { }

        public JsonContentConverter(JsonProtocol protocol)
        {
            this.protocol = protocol;
        }

        public JsonContentConverter(JsonSerializer serializer)
        {
            protocol = new JsonProtocol(serializer);
        }

        public HttpContent ToContent<T>(T value)
        {
            var content = new StringContent(protocol.Serialize(value));
            content.Headers.ContentType = new MediaTypeHeaderValue("application/json");
            return content;
        }

        public async Task<T> FromContent<T>(HttpContent content)
        {
            using var stream = await content.ReadAsStreamAsync();
            using var reader = new StreamReader(stream);
            return protocol.DeserializeStream<T>(reader);
        }
    }
}
