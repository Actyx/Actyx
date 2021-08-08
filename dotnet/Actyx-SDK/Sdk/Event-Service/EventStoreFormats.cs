using System.Collections.Generic;
using Actyx.Sdk.Utils;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx
{
    public static class EventStoreSerializer
    {
        public static JsonSerializer Create(bool pretty = true) => DefaultJsonSerializer.Create(new List<JsonConverter>
        {
            JsonSubtypesConverterBuilder
                .Of(typeof(IResponseMessage), "type")
                .RegisterSubtype<OffsetsOnWire>("offsets")
                .RegisterSubtype<EventOnWire>("event")
                .RegisterSubtype<DiagnosticOnWire>("diagnostic")
                .SerializeDiscriminatorProperty()
                .Build(),
            JsonSubtypesConverterBuilder
                .Of(typeof(ISubscribeMonotonicResponse), "type")
                .RegisterSubtype<SubscribeMonotonicEventResponse>("event")
                .RegisterSubtype<SubscribeMonotonicOffsetsResponse>("offsets")
                .RegisterSubtype<SubscribeMonotonicTimeTravelResponse>("timeTravel")
                .SerializeDiscriminatorProperty()
                .Build(),
        }, pretty);
    }

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

    public class PublishSucceeded
    {
        public EventKey Key { get; protected set; }
        public string AppId { get; protected set; }
    }

    public interface IEventDraft
    {
        IEnumerable<string> Tags { get; }
        // Must be JSON-Serializable.
        object Payload { get; }
    }

    public struct EventDraft : IEventDraft
    {
        public IEnumerable<string> Tags { get; set; }
        public object Payload { get; set; }
    }

    public class EventPublishMetadata
    {
        public ulong Lamport { get; set; }
        public ulong Offset { get; set; }
        public ulong Timestamp { get; set; }
        public string Stream { get; set; }
    }

    public interface IResponseMessage { }

    public class OffsetsOnWire : IResponseMessage
    {
        public OffsetMap Offsets { get; set; }
    }

    public enum DiagnosticSeverity { Warning, Error }

    public class DiagnosticOnWire : IResponseMessage
    {
        public DiagnosticSeverity Severity;
        public string Message;
    }

    // Internal event class, 1:1 correspondence with wire format
    public class EventOnWire : EventPublishMetadata, IResponseMessage
    {
        public string AppId { get; set; }
        public IEnumerable<string> Tags { get; set; }
        public JToken Payload { get; set; }
    }

    public interface ISubscribeMonotonicResponse { }

    public class SubscribeMonotonicOffsetsResponse : ISubscribeMonotonicResponse
    {
        public OffsetMap Offsets { get; set; }
    }

    public class SubscribeMonotonicEventResponse : ISubscribeMonotonicResponse
    {
        public string AppId { get; set; }
        public IEnumerable<string> Tags { get; set; }
        public JToken Payload { get; set; }
        public ulong Lamport { get; set; }
        public ulong Offset { get; set; }
        public ulong Timestamp { get; set; }
        public string Stream { get; set; }
        public bool CaughtUp { get; set; }
    }

    public class SubscribeMonotonicTimeTravelResponse : ISubscribeMonotonicResponse
    {
        public EventKey NewStart { get; set; }
    }

    public class PublishResponse
    {
        public IEnumerable<EventPublishMetadata> Data { get; set; }
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


    public class OffsetMap : Dictionary<string, ulong>
    {
        public OffsetMap() : base() { }

        // Just type alias. TODO Is this ideal? (Maybe use an immutable dict)
        public OffsetMap(IDictionary<string, ulong> dictionary) : base(dictionary) { }
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
