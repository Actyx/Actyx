using System;
using System.Net.Http;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;

namespace Actyx.Sdk.AxHttpClient
{
    public class AxHttpClient : IAxHttpClient
    {
        private readonly HttpClient httpClient;
        private readonly UriBuilder uriBuilder;
        private readonly AppManifest manifest;
        private readonly JsonContentConverter converter;
        public string Token { get; private set; }
        public NodeId NodeId { get; private set; }
        public string AppId => manifest.AppId;

        private AxHttpClient(HttpClient httpClient, string baseUrl, AppManifest manifest, JsonContentConverter converter)
        {
            ThrowIf.Argument.IsNull(baseUrl, nameof(baseUrl));
            ThrowIf.Argument.IsNull(manifest, nameof(manifest));
            ThrowIf.Argument.IsNull(converter, nameof(converter));

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

            this.httpClient = httpClient;
            this.manifest = manifest;
            this.converter = converter;
        }

        public static async Task<AxHttpClient> Create(string baseUrl, AppManifest manifest, JsonContentConverter converter)
        {
            var httpClient = new HttpClient();
            var client = new AxHttpClient(httpClient, baseUrl, manifest, converter)
            {
                NodeId = await GetNodeId(httpClient, new Uri(baseUrl)),
            };
            client.Token = (await GetToken(httpClient, client.uriBuilder.Uri, manifest, converter)).Token;

            return client;
        }

        public static async Task<NodeId> GetNodeId(HttpClient httpClient, Uri baseUri)
        {
            var uri = baseUri + HttpApiPath.NODE_ID_SEG;
            var nodeIdResponse = await httpClient.GetAsync(uri);
            await nodeIdResponse.EnsureSuccessStatusCodeCustom();
            var nodeId = await nodeIdResponse.Content.ReadAsStringAsync();
            return new NodeId(nodeId);
        }

        public Task<HttpResponseMessage> Post<T>(string path, T payload, bool xndjson = false) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var uri = MkApiUrl(path);
                var request = new HttpRequestMessage(HttpMethod.Post, uri);
                request.Headers.Add("Accept", xndjson ? "application/x-ndjson" : "application/json");
                request.Headers.Add("Authorization", $"Bearer {Token}");
                request.Content = converter.ToContent(payload);
                return httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
            });

        public Task<HttpResponseMessage> Get(string path) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var uri = MkApiUrl(path);
                var request = new HttpRequestMessage(HttpMethod.Get, uri);
                request.Headers.Add("Authorization", $"Bearer {Token}");
                request.Headers.Add("Accept", "application/json");
                return httpClient.SendAsync(request);
            });

        public static async Task<AuthenticationResponse> GetToken(HttpClient httpClient, Uri baseUri, AppManifest manifest, JsonContentConverter converter)
        {
            var response = await httpClient.PostAsync(baseUri + HttpApiPath.AUTH_SEG, converter.ToContent(manifest));
            await response.EnsureSuccessStatusCodeCustom();
            return await converter.FromContent<AuthenticationResponse>(response.Content);
        }

        private string MkApiUrl(string path) => uriBuilder.Uri + path;

        private async Task<HttpResponseMessage> FetchWithRetryOnUnauthorized(Func<Task<HttpResponseMessage>> request)
        {
            var response = await request();
            if (response.IsUnauthorized())
            {
                Token = (await GetToken(httpClient, uriBuilder.Uri, manifest, converter)).Token;
                response = await request();
            }

            return response;
        }
    }
}
