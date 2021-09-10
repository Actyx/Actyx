using System.Collections.Generic;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Formats
{
    public struct EventChunk
    {
        public EventChunk(OffsetMap lowerBound, OffsetMap upperBound, IList<ActyxEvent<JToken>> events)
        {
            LowerBound = lowerBound;
            UpperBound = upperBound;
            Events = events;
        }

        public OffsetMap LowerBound { private set; get; }

        public OffsetMap UpperBound { private set; get; }

        public IList<ActyxEvent<JToken>> Events { private set; get; }
    }
}
