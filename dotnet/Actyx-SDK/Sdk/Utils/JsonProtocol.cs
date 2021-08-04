using System.IO;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Utils
{
    public class JsonProtocol
    {
        private readonly JsonSerializer serializer;
        public JsonProtocol(JsonSerializer serializer)
        {
            this.serializer = serializer;
        }

        public T Deserialize<T>(string s) =>
            DeserializeStream<T>(new StringReader(s));

        public T DeserializeStream<T>(TextReader reader)
        {
            using var jsonReader = new JsonTextReader(reader);
            return serializer.Deserialize<T>(jsonReader);
        }

        public T DeserializeJson<T>(JToken json) =>
            json.ToObject<T>(serializer);

        public string Serialize<T>(T value)
        {
            using var writer = new StringWriter();
            SerializeStream(writer, value);
            return writer.ToString();
        }

        public void SerializeStream<T>(TextWriter writer, T value) =>
            serializer.Serialize(writer, value);

        public JToken SerializeJson<T>(T value) =>
            JToken.FromObject(value, serializer);
    }
}
