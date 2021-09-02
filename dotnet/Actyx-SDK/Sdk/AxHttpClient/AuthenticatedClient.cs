using System;
using System.Net.Http;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Utils.Extensions;

namespace Actyx.Sdk.AxHttpClient
{
    /// Intercepts `Fetch()` calls and inserts Authentication headers. Refreshes token if expired.
    public class AuthenticatedClient : AxHttpClient
    {
        private readonly AppManifest manifest;
        private readonly Uri authUri;
        private string token = null;
        public AuthenticatedClient(
            AppManifest manifest,
            Uri baseUri,
            Uri authUri,
            JsonContentConverter converter) : base(baseUri, converter)
        {
            ThrowIf.Argument.IsNull(manifest, nameof(manifest));
            this.manifest = manifest;
            this.authUri = authUri;
        }

        override public async Task<HttpResponseMessage> DoFetch(HttpRequestMessage request)
        {
            token ??= await GetToken(); // first request
            AddAuthorization(request);
            var response = await Fetch(request);
            if (response.IsUnauthorized())
            {
                token = await GetToken(); // token expired
                AddAuthorization(request);
                response = await Fetch(request);
            }
            return response;
        }

        public async Task<string> GetToken()
        {
            var request = new HttpRequestMessage(HttpMethod.Post, authUri);
            request.Headers.Add("Accept", "application/json");
            request.Content = converter.ToContent(manifest);
            var response = await Fetch(request);
            return (await converter.FromContent<AuthenticationResponse>(response.Content)).Token;

        }
        // (await Post<AppManifest, AuthenticationResponse>(HttpApiPath.AUTH_SEG, manifest)).Token;

        private void AddAuthorization(HttpRequestMessage request) =>
            request.Headers.Add("Authorization", $"Bearer {token}");
    }
}
