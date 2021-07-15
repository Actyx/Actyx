using System.Collections.Generic;
using Actyx;
using Newtonsoft.Json.Linq;

namespace Sdk.Tests.Helpers
{
    class TestEvent : IEventDraft
    {
        public IEnumerable<string> Tags => Constants.Tags;

        public JToken Payload { get; set; }

        public TestEvent(string payload)
        {
            Payload = payload;
        }
    }
}
