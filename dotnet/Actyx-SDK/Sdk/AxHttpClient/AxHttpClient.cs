using System;
using System.Net.Http;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json;

namespace Actyx.Sdk.AxHttpClient
{
    public class AxHttpClientException : Exception
    {
        public AxHttpClientException(string message, Exception inner) : base(message, inner) { }
        public AxHttpClientException(string message) : base(message) { }
    }

    public class AxHttpClient : IAxHttpClient
    {
        private readonly HttpClient httpClient;
        private readonly UriBuilder uriBuilder;
        private readonly AppManifest manifest;
        private readonly JsonSerializer serializer;
        public string Token { get; private set; }
        public NodeId NodeId { get; private set; }
        public string AppId => manifest.AppId;

        private AxHttpClient(HttpClient httpClient, string baseUrl, AppManifest manifest, JsonSerializer serializer)
        {
            ThrowIf.Argument.IsNull(baseUrl, nameof(baseUrl));
            ThrowIf.Argument.IsNull(manifest, nameof(manifest));
            ThrowIf.Argument.IsNull(serializer, nameof(serializer));

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
            this.serializer = serializer;
        }

        public static async Task<AxHttpClient> Create(string baseUrl, AppManifest manifest, JsonSerializer serializer)
        {
            var httpClient = new HttpClient();
            var client = new AxHttpClient(httpClient, baseUrl, manifest, serializer)
            {
                NodeId = await GetNodeId(httpClient, new Uri(baseUrl)),
            };
            client.Token = (await GetToken(httpClient, client.uriBuilder.Uri, manifest, serializer)).Token;

            return client;
        }

        public static async Task<NodeId> GetNodeId(HttpClient httpClient, Uri baseUri)
        {
            var uri = baseUri + HttpApiPath.NODE_ID_SEG;
            try
            {
                var nodeIdResponse = await httpClient.GetAsync(uri);
                await nodeIdResponse.EnsureSuccessStatusCodeCustom();
                var nodeId = await nodeIdResponse.Content.ReadAsStringAsync();
                return new NodeId(nodeId);
            }
            catch (Exception e)
            {
                throw new AxHttpClientException($"Error GETting {uri}", e);
            }
        }

        public Task<HttpResponseMessage> Post<T>(string path, T payload, bool xndjson = false) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var uri = MkApiUrl(path);
                try
                {
                    var request = new HttpRequestMessage(HttpMethod.Post, uri);
                    request.Headers.Add("Accept", xndjson ? "application/x-ndjson" : "application/json");
                    request.Headers.Add("Authorization", $"Bearer {Token}");
                    request.Content = new JsonContent(payload, serializer);
                    return httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
                }
                catch (Exception e)
                {
                    throw new AxHttpClientException($"Error POSTing to {uri}", e);
                }
            });

        public Task<HttpResponseMessage> Get(string path) =>
            FetchWithRetryOnUnauthorized(() =>
            {
                var uri = MkApiUrl(path);
                try
                {
                    var request = new HttpRequestMessage(HttpMethod.Get, uri);
                    request.Headers.Add("Authorization", $"Bearer {Token}");
                    request.Headers.Add("Accept", "application/json");
                    return httpClient.SendAsync(request);
                }
                catch (Exception e)
                {
                    throw new AxHttpClientException($"Error GETting {uri}", e);
                }
            });

        public static async Task<AuthenticationResponse> GetToken(HttpClient httpClient, Uri baseUri, AppManifest manifest, JsonSerializer serializer)
        {
            var response = await httpClient.PostAsync(baseUri + HttpApiPath.AUTH_SEG, new JsonContent(manifest, serializer));
            await response.EnsureSuccessStatusCodeCustom();
            return await response.Content.ReadFromJsonAsync<AuthenticationResponse>(EventStore.Protocol);
        }

        private string MkApiUrl(string path) => uriBuilder.Uri + path;

        private async Task<HttpResponseMessage> FetchWithRetryOnUnauthorized(Func<Task<HttpResponseMessage>> request)
        {
            var response = await request();
            if (response.IsUnauthorized())
            {
                Token = (await GetToken(httpClient, uriBuilder.Uri, manifest, serializer)).Token;
                response = await request();
            }

            return response;
        }
    }
}
