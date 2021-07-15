using System.Collections.Generic;

namespace Actyx.Sdk.Formats
{
    public class ActyxEventMetadata
    {
        private static int MaxLamportLength => uint.MaxValue.ToString("D").Length;
        private static string MkEventId(ulong lamport, string stream)
        {
            var lam = lamport.ToString("D");
            return $"{lam.PadLeft(MaxLamportLength, '0')}/{stream}";
        }

        public ActyxEventMetadata(EventOnWire ev, NodeId nodeId)
        {
            IsLocalEvent = nodeId.IsOwn(ev.Stream);
            Tags = ev.Tags;
            TimestampMicros = ev.Timestamp;
            Lamport = ev.Lamport;
            EventId = MkEventId(ev.Lamport, ev.Stream);
            AppId = ev.AppId;
            Stream = ev.Stream;
            Offset = ev.Offset;
        }

        // Was this event written by the very node we are running on?
        public bool IsLocalEvent { private set; get; }

        // Tags belonging to the event.
        public IEnumerable<string> Tags { private set; get; }

        // Time since Unix Epoch **in Microseconds**!
        // FIXME should use dotnet Duration type or something
        public ulong TimestampMicros { private set; get; }

        // FIXME should offer Dotnet Date type
        //  timestampAsDate: () => Date

        // Lamport timestamp of the event. Cf. https://en.wikipedia.org/wiki/Lamport_timestamp
        public ulong Lamport { private set; get; }

        // A unique identifier for the event.
        // Every event has exactly one eventId which is unique to it, guaranteed to not collide with any other event.
        // Events are *sorted* based on the eventId by Actyx: For a given event, all later events also have a higher eventId according to simple string-comparison.
        public string EventId { private set; get; }

        // App id of the event
        public string AppId { private set; get; }

        // Stream this event belongs to
        public string Stream { private set; get; }

        // Offset of this event inside its stream
        public long Offset { private set; get; }
    }
}
