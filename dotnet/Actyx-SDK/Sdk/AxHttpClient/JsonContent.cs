using System.IO;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Threading.Tasks;
using Newtonsoft.Json;

namespace Actyx.Sdk.AxHttpClient
{
    public class JsonContent : HttpContent
    {
        private readonly JsonSerializer serializer;
        private readonly object value;

        public JsonContent(object value, JsonSerializer serializer)
        {
            this.value = value;
            this.serializer = serializer;
            Headers.ContentType = new MediaTypeHeaderValue("application/json");
        }

        protected override Task SerializeToStreamAsync(Stream stream, TransportContext context)
        {
            var writeBOM = false;
            using var writer = new StreamWriter(stream, new UTF8Encoding(writeBOM), 1024, leaveOpen: true);
            using var jsonWriter = new JsonTextWriter(writer);
            serializer.Serialize(jsonWriter, value);
            return Task.CompletedTask;
        }

        protected override bool TryComputeLength(out long length)
        {
            // ???
            length = -1;
            return false;
        }
    }
}
