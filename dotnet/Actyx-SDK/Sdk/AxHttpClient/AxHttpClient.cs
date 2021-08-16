using System;
using System.Linq;
using System.Net.Http;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.AxHttpClient
{
    /// Provides typed, JSON-based access to resources via relative paths.
    public class AxHttpClient : IAxHttpClient
    {
        private readonly HttpClient httpClient;
        protected readonly Uri baseUri;
        protected readonly JsonContentConverter converter;

        public AxHttpClient(
            Uri baseUri,
            JsonContentConverter converter)
        {
            ThrowIf.Argument.IsNull(baseUri, nameof(baseUri));
            if (!baseUri.Scheme.Equals("http"))
            {
                throw new ArgumentException($"Only http scheme allowed. Received '{baseUri.Scheme}'.");
            }
            if (!baseUri.IsAbsoluteUri)
            {
                throw new ArgumentException($"`baseUri` needs to be an absolute. Received '{baseUri}'.");
            }

            ThrowIf.Argument.IsNull(converter, nameof(converter));

            this.baseUri = baseUri;
            this.converter = converter;

            httpClient = new HttpClient();
        }

        public virtual Task<HttpResponseMessage> DoFetch(HttpRequestMessage request) =>
            Fetch(request);


        public async Task<HttpResponseMessage> Fetch(HttpRequestMessage request)
        {
            var response = await httpClient.SendAsync(request, HttpCompletionOption.ResponseHeadersRead);
            await response.EnsureSuccessStatusCodeCustom();
            return response;
        }

        public async Task<Res> Get<Res>(string path)
        {
            var uri = new Uri(baseUri, path);
            var request = new HttpRequestMessage(HttpMethod.Get, uri);
            request.Headers.Add("Accept", "application/json");
            var response = await DoFetch(request);
            return await converter.FromContent<Res>(response.Content);
        }

        public async Task<Res> Post<Req, Res>(string path, Req payload)
        {
            var uri = new Uri(baseUri, path);
            var request = new HttpRequestMessage(HttpMethod.Post, uri);
            request.Headers.Add("Accept", "application/json");
            request.Content = converter.ToContent(payload);
            var response = await DoFetch(request);
            return await converter.FromContent<Res>(response.Content);
        }

        public IObservable<Res> Stream<Req, Res>(string path, Req payload)
        {
            var uri = new Uri(baseUri, path);
            var request = new HttpRequestMessage(HttpMethod.Post, uri);
            request.Headers.Add("Accept", "application/x-ndjson");
            request.Content = converter.ToContent(payload);
            return Observable
                .FromAsync(() => DoFetch(request))
                .SelectMany(response =>
                    Observable
                        .FromAsync(async () => await response.EnsureSuccessStatusCodeCustom())
                        .SelectMany(_ => response.Content!
                            .ReadFromNdjsonAsync().ToObservable()
                            .TrySelect(EventStore.Protocol.DeserializeJson<Res>, LogDecodingError))
                );
        }

        private static void LogDecodingError(JToken json, Exception error) =>
            Console.Error.WriteLine($"Error decoding {json}: {error.Message}");
    }
}
