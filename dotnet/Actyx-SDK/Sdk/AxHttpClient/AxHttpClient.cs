using System;
using System.Net.Http;
using System.Text;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json;

namespace Actyx.Sdk.AxHttpClient
{
    public class AxHttpClient : IAxHttpClient
    {
        private static readonly HttpClient httpClient;
        static AxHttpClient()
        {
            httpClient = new HttpClient();
        }

        public static async Task<AxHttpClient> Create(string baseUrl, AppManifest manifest)
        {

            var that = new AxHttpClient(baseUrl, manifest);
            var nodeIdResponse = await httpClient.GetAsync(that.MkApiUrl(HttpApiPath.NODE_ID_SEG));
            await nodeIdResponse.EnsureSuccessStatusCodeCustom();
            var nodeId = await nodeIdResponse.Content.ReadAsStringAsync();
            that.NodeId = new NodeId(nodeId);
            that.token = (await GetToken(that.uriBuilder.Uri, manifest)).Token;

            return that;
        }

        private readonly UriBuilder uriBuilder;
        private readonly AppManifest manifest;
        private string token;

        public NodeId NodeId { get; private set; }

        public string AppId => manifest.AppId;

        private AxHttpClient(string baseUrl, AppManifest manifest)
        {
            this.manifest = manifest;
            if (!Uri.TryCreate(baseUrl, UriKind.Absolute, out Uri uri))
            {
                throw new ArgumentException($"Base url needs to be an absolute, i.e. 'http://localhost:4454'. Received '{baseUrl}'.");
            }
            if (!uri.Scheme.Equals("http"))
            {
                throw new ArgumentException($"Only http scheme allowed, i.e. 'http://localhost:4454'. Received '{baseUrl}'.");
            }
            uriBuilder = new UriBuilder(uri)
            {
                Path = HttpApiPath.API_V2_PATH,
            };
        }

        public Task<HttpResponseMessage> Post<T>(string path, T data, bool xndjson = false) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var request = new HttpRequestMessage(HttpMethod.Post, MkApiUrl(path));
                request.Headers.Add("Accept", xndjson ? "application/x-ndjson" : "application/json");
                request.Headers.Add("Authorization", $"Bearer {token}");
                request.Content = CreateJsonContent(data);
                return httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
            });

        public Task<HttpResponseMessage> Get(string path) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var request = new HttpRequestMessage(HttpMethod.Get, MkApiUrl(path));
                request.Headers.Add("Authorization", $"Bearer {token}");
                request.Headers.Add("Accept", "application/json");
                return httpClient.SendAsync(request);
            });

        private static async Task<AuthenticationResponse> GetToken(Uri baseUri, AppManifest manifest)
        {
            var response = await httpClient.PostAsync(baseUri + HttpApiPath.AUTH_SEG, CreateJsonContent(manifest));
            await response.EnsureSuccessStatusCodeCustom();
            return await response.Content.ReadFromJsonAsync<AuthenticationResponse>();
        }

        private static StringContent CreateJsonContent<T>(T value)
        {
            var json = JsonConvert.SerializeObject(value, HttpContentExtensions.JsonSettings);
            var result = new StringContent(json, Encoding.UTF8, "application/json");

            return result;
        }

        private string MkApiUrl(string path) => uriBuilder.Uri + path;

        private async Task<HttpResponseMessage> FetchWithRetryOnUnauthorized(Func<Task<HttpResponseMessage>> request)
        {
            var response = await request();
            if (response.IsUnauthorized())
            {
                token = (await GetToken(uriBuilder.Uri, manifest)).Token;
                response = await request();
            }

            return response;
        }
    }
}
