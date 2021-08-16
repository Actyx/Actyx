using System.Collections.Generic;
using Actyx;

namespace Sdk.IntegrationTests.Helpers
{
    class TestEvent : IEventDraft
    {
        public IEnumerable<string> Tags { get; set; }

        public object Payload { get; set; }

        public TestEvent(object payload, IEnumerable<string> tags)
        {
            Payload = payload;
            Tags = tags;
        }
    }
}
