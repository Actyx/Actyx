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
                string error;
                if (that.StatusCode == System.Net.HttpStatusCode.NotFound)
                {
                    error = $"URI Not Found!{that.RequestMessage.RequestUri}";
                }
                else if (that.StatusCode == System.Net.HttpStatusCode.Unauthorized)
                {
                    error = $"Unauthorized!{that.RequestMessage.RequestUri}";
                }
                else
                {
                    error = await that.Content.ReadAsStringAsync();
                }

                throw new Exception(that.ReasonPhrase + " " + error);
            }
        }

        public static bool IsUnauthorized(this HttpResponseMessage that) =>
            !that.IsSuccessStatusCode && that.StatusCode == System.Net.HttpStatusCode.Unauthorized;
    }
}
