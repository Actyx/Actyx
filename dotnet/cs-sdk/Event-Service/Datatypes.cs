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

    struct RetrievedSnapshot
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

    struct EventMetadata
    {
	public UInt64 Timestamp { get; set; }

	public string[] Tags { get; set; }

	public string AppId { get; set; }
    }

    class State : ISuttMessage
    {
	public string Type { get; } = "state";

	[JsonProperty("snapshot")]
	public RetrievedSnapshot Snapshot { get; protected set; }
    }

    class Event : ISuttMessage
    {

	public string Type { get; } = "event";

	// Only relevant if the event was retrieved via subscribeUntilTimeTravel endpoint.
	[JsonProperty("caughtUp")]
	public bool CaughtUp { get; protected set; } = true;

	[JsonProperty("key")]
	public EventKey Key { get; protected set; }

	[JsonProperty("meta")]
	public EventMetadata Meta { get; protected set; }

	[JsonProperty("payload")]
	public JObject Payload { get; protected set; }
    }

    class TimeTravel : ISuttMessage
    {
	public string Type { get; } = "timeTravel";

	[JsonProperty("newStart")]
	public EventKey NewStart { get; protected set; }
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
