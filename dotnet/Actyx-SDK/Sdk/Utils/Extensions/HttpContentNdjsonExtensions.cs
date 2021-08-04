using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Utils.Extensions
{
    internal static class HttpContentNdjsonExtensions
    {

        public static async IAsyncEnumerable<JToken> ReadFromNdjsonAsync(this HttpContent content)
        {
            if (content is null)
            {
                throw new ArgumentNullException(nameof(content));
            }

            string mediaType = content.Headers.ContentType?.MediaType;

            if (mediaType is null || !mediaType.Equals("application/x-ndjson", StringComparison.OrdinalIgnoreCase))
            {
                throw new NotSupportedException($"Tried to read 'application/x-ndjson', but got '{mediaType}'");
            }

            using var contentStream = await content.ReadAsStreamAsync().ConfigureAwait(false);
            using var contentStreamReader = new StreamReader(contentStream);
            while (!contentStreamReader.EndOfStream)
            {
                var line = await contentStreamReader.ReadLineAsync().ConfigureAwait(false);
                if (string.IsNullOrEmpty(line))
                {
                    continue;
                }
                yield return JToken.Parse(line);
            }
        }
    }
}
