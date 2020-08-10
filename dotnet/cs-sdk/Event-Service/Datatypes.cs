using Newtonsoft.Json;
using Newtonsoft.Json.Linq;


namespace Actyx {

    interface OffsetMap : IDictionary<string, Int64> {
	// Just a type alias
    }
    
    class EventKey {
	
	public Int64 Lamport { get; set; }

	public string Stream { get; set; }

	public Int64 Offset { get; set; }
    }

    class EventMetadata {
	
	public Int64 Timestamp { get; set; }

	public string[] Tags { get; set; }

	public string AppId { get; set; }

    }

    class Event
    {
	public EventKey key { get; set; }

	public EventMetadata meta { get; set; }
	
	public JObject payload { get; set; }
    }

    
}
