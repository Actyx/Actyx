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
    
    class EventKey {

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

	public bool caughtUp { get; set; } = true;
	
	public EventKey key { get; set; }

	public EventMetadata meta { get; set; }

	public JObject payload { get; set; }
    }

    class State : ISuttMessage
    {
	
	public string Type { get; } = "state";

	// snapshot: {
	// 	compression: 'none'|'deflate',
	// 	data: string // base64-encoded unless 'none' compression
	// }
    }

    class TimeTravel : ISuttMessage
    {
	
	public string Type { get; } = "timeTravel";
	
	public EventKey newStart { get; set; }
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
