using System.Collections.Generic;
using Actyx;

namespace Sdk.Tests.Helpers
{
    class TestEvent : IEventDraft
    {
        public IEnumerable<string> Tags => Constants.Tags;

        public object Payload { get; set; }

        public TestEvent(string payload)
        {
            Payload = payload;
        }
    }
}
