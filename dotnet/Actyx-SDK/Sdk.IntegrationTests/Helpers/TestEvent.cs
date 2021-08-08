using System.Collections.Generic;
using Actyx;

namespace Sdk.IntegrationTests.Helpers
{
    class TestEvent : IEventDraft
    {
        public IEnumerable<string> Tags => Constants.Tags;

        public object Payload { get; set; }

        public TestEvent(object payload)
        {
            Payload = payload;
        }
    }
}
