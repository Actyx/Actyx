using System.Collections.Generic;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx
{
    public enum EventsOrder
    {
        /// Events are sorted by ascending Lamport timestamp and stream ID, which defines a
        /// total order.
        Asc,
        /// Events are sorted by descending Lamport timestamp and descending stream ID,
        /// which is the exact reverse of the `Asc` ordering.
        Desc,
        /// Events are sorted within each stream by ascending Lamport timestamp, with events
        /// from different streams interleaved in an undefined order.
        StreamAsc,
    }
    public static class Extensions
    {
        public static string ToWireString(this EventsOrder order)
        {
            return order switch
            {
                EventsOrder.Asc => "asc",
                EventsOrder.Desc => "desc",
                EventsOrder.StreamAsc => "stream-asc",
                _ => "asc",
            };
        }
    }

    public class PublishSucceeded
    {
        [JsonProperty("key")]
        public EventKey Key { get; protected set; }

        [JsonProperty("appId")]
        public string AppId { get; protected set; }
    }

    public interface IEventDraft
    {
        [JsonProperty("tags")]
        IEnumerable<string> Tags { get; }

        // Must be JSON-Serializable.
        [JsonProperty("payload")]
        object Payload { get; }
    }

    public struct EventDraft : IEventDraft
    {
        public IEnumerable<string> Tags { get; set; }

        public object Payload { get; set; }
    }

    public interface ISubscribeMonotonicMessageVisitor
    {
        void Visit(State stateMsg);

        void Visit(Event eventMsg);

        void Visit(TimeTravel timeTravelMsg);
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface ISubscribeMonotonicMessage
    {
        string Type { get; }

        void Accept(ISubscribeMonotonicMessageVisitor visitor);
    }

    public enum SnapshotCompression
    {
        None,
        Deflate,
    }

    public struct RetrievedSnapshot
    {
        public SnapshotCompression Compression { get; set; }

        // Base64-encoded bytes if a compression other than None is set.
        public string Data { get; set; }
    }

    public struct EventKey
    {
        public ulong Lamport { get; set; }

        public string Stream { get; set; }

        public ulong Offset { get; set; }
    }

    public struct EventMetadata
    {
        public ulong Timestamp { get; set; }

        public string[] Tags { get; set; }

        public string AppId { get; set; }
    }

    public class State : ISubscribeMonotonicMessage
    {
        public string Type { get; } = "state";

        [JsonProperty("snapshot")]
        public RetrievedSnapshot Snapshot { get; protected set; }

        public void Accept(ISubscribeMonotonicMessageVisitor handler)
        {
            handler.Visit(this);
        }
    }

    public class Event : ISubscribeMonotonicMessage
    {
        public string Type { get; } = "event";

        // Only relevant if the event was retrieved via subscribe_monotonic endpoint.
        [JsonProperty("caughtUp")]
        public bool CaughtUp { get; protected set; } = true;

        [JsonProperty("key")]
        public EventKey Key { get; protected set; }

        [JsonProperty("meta")]
        public EventMetadata Meta { get; protected set; }

        [JsonProperty("payload")]
        public JToken Payload { get; protected set; }

        public void Accept(ISubscribeMonotonicMessageVisitor handler)
        {
            handler.Visit(this);
        }
    }

    public class TimeTravel : ISubscribeMonotonicMessage
    {
        public string Type { get; } = "timeTravel";

        [JsonProperty("newStart")]
        public EventKey NewStart { get; protected set; }

        public void Accept(ISubscribeMonotonicMessageVisitor handler)
        {
            handler.Visit(this);
        }
    }

    public class OffsetMap : Dictionary<string, ulong>
    {
        // Just type alias. TODO Is this ideal? (Maybe use an immutable dict)
    }

    public class OffsetsResponse
    {
        public OffsetMap Present { get; set; }

        // NOT an offset-map. Rather it contains offsets between offsets :)
        public OffsetMap ToReplicate { get; set; }
    }

    public interface IEventSelection
    {
        string ToAql();
    }
}
