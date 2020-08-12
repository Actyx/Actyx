using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;

namespace Actyx {

    public enum EventsOrder
    {
	Lamport,
	LamportReverse,
	SourceOrdered,
    }

    public static class Extensions
    {
        public static string ToWireString(this EventsOrder order)
        {
	    switch (order) {
		case EventsOrder.Lamport: return "lamport";
		case EventsOrder.LamportReverse: return "lamport-reverse";
		case EventsOrder.SourceOrdered: return "source-ordered";
		default: return "lamport";
	    }
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
	IEnumerable<string> Tags { get; }

	// Must be JSON-Serializable.
	Object Payload { get; }
    }

    public interface ISuttMessageVisitor
    {
	void Visit(State stateMsg);

	void Visit(Event eventMsg);

	void Visit(TimeTravel timeTravelMsg);
    }

    [JsonConverter(typeof(JsonSubtypes), "Type")]
    public interface ISuttMessage
    {
	string Type { get; }

	void Accept(ISuttMessageVisitor visitor);
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
	public UInt64 Lamport { get; set; }

	public string Stream { get; set; }

	public UInt64 Offset { get; set; }
    }

    public struct EventMetadata
    {
	public UInt64 Timestamp { get; set; }

	public string[] Tags { get; set; }

	public string AppId { get; set; }
    }

    public class State : ISuttMessage
    {
	public string Type { get; } = "state";

	[JsonProperty("snapshot")]
	public RetrievedSnapshot Snapshot { get; protected set; }

	public void Accept(ISuttMessageVisitor handler) {
	    handler.Visit(this);
	}
    }

    public class Event : ISuttMessage
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

	public void Accept(ISuttMessageVisitor handler) {
	    handler.Visit(this);
	}
    }

    public class TimeTravel : ISuttMessage
    {
	public string Type { get; } = "timeTravel";

	[JsonProperty("newStart")]
	public EventKey NewStart { get; protected set; }

	public void Accept(ISuttMessageVisitor handler) {
	    handler.Visit(this);
	}
    }

    public class EventV1
    {

	public UInt64 Lamport { get; set; }

	// public string Stream { get; set; }

	public UInt64 Offset { get; set; }

	public UInt64 Timestamp { get; set; }

	public JObject payload { get; set; }
    }
}
