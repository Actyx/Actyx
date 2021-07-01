using System;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Formats
{
    public class ActyxEvent
    {

        public ActyxEventMetadata Meta { private set; get; }

        public JToken Payload { private set; get; }

        public static Func<EventOnWire, ActyxEvent> From(NodeId nodeId) => ev =>
            new ActyxEvent
            {
                Meta = new ActyxEventMetadata(ev, nodeId),
                Payload = ev.Payload,
            };
    }
}
