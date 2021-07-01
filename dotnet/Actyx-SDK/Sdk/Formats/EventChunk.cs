using System.Collections.Generic;

namespace Actyx.Sdk.Formats
{
    public struct EventChunk
    {
        public EventChunk(OffsetMap lowerBound, OffsetMap upperBound, IList<ActyxEvent> events)
        {
            LowerBound = lowerBound;
            UpperBound = upperBound;
            Events = events;
        }

        public OffsetMap LowerBound { private set; get; }

        public OffsetMap UpperBound { private set; get; }

        public IList<ActyxEvent> Events { private set; get; }
    }
}
