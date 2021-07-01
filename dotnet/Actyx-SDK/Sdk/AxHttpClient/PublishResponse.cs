using System.Collections.Generic;

namespace Actyx.Sdk.AxHttpClient
{
    public class PublishResponse
    {
        public IEnumerable<EventPublishMetadata> Data { get; set; }
    }
}
