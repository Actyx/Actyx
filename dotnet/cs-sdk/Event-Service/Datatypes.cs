using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System;
using System.Collections.Generic;

namespace Actyx {

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

    class Event
    {
	public EventKey key { get; set; }

	public EventMetadata meta { get; set; }

	public JObject payload { get; set; }
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
