using System;
using System.Net.Http;
using System.Threading.Tasks;

namespace Actyx.Sdk.Utils.Extensions
{
    public static class HttpResponseMessageExtensions
    {
        public static async Task EnsureSuccessStatusCodeCustom(this HttpResponseMessage that)
        {
            if (!that.IsSuccessStatusCode)
            {
                string error = that.StatusCode switch
                {
                    System.Net.HttpStatusCode.NotFound or System.Net.HttpStatusCode.Unauthorized => that.ReasonPhrase,
                    _ => await that.Content.ReadAsStringAsync(),
                };
                throw new Exception($"Error requesting {that.RequestMessage.RequestUri}: {error}");
            }
        }

        public static bool IsUnauthorized(this HttpResponseMessage that) =>
            !that.IsSuccessStatusCode && that.StatusCode == System.Net.HttpStatusCode.Unauthorized;
    }
}
