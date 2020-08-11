using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;

namespace Actyx {

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface ISuttMessage
    {
	string Type { get; }
    }


    enum SnapshotCompression
    {
	None,
	Deflate,
    }

    class RetrievedSnapshot
    {
	public SnapshotCompression Compression { get; set; }

	// Base64-encoded bytes if a compression other than None is set.
	public string Data { get; set; }
    }

    struct EventKey
    {
	public UInt64 Lamport { get; set; }

	public string Stream { get; set; }

	public UInt64 Offset { get; set; }
    }

    class EventMetadata {

	public UInt64 Timestamp { get; set; }

	public string[] Tags { get; set; }

	public string AppId { get; set; }

    }

    class Event : ISuttMessage
    {

	public string Type { get; } = "event";

	// Only relevant if the event was retrieved via subscribeUntilTimeTravel endpoint.
	public bool CaughtUp { get; set; } = true;

	public EventKey Key { get; set; }

	public EventMetadata Meta { get; set; }

	public JObject Payload { get; set; }
    }

    class State : ISuttMessage
    {

	public string Type { get; } = "state";

	public RetrievedSnapshot Snapshot { get; set; }
    }

    class TimeTravel : ISuttMessage
    {
	public string Type { get; } = "timeTravel";

	public EventKey NewStart { get; set; }
    }

    class EventV1
    {

	public UInt64 Lamport { get; set; }

	// public string Stream { get; set; }

	public UInt64 Offset { get; set; }

	public UInt64 Timestamp { get; set; }

	public JObject payload { get; set; }
    }
}
