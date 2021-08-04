using System.IO;
using System.Net.Http;
using System.Threading.Tasks;

namespace Actyx.Sdk.Utils.Extensions
{
    public static class HttpContentExtensions
    {
        public static async Task<T> ReadFromJsonAsync<T>(this HttpContent content, JsonProtocol protocol)
        {
            using var stream = await content.ReadAsStreamAsync();
            using var reader = new StreamReader(stream);
            return protocol.DeserializeStream<T>(reader);
        }
    }
}
