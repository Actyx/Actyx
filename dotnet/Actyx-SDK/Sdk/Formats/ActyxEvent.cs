using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Formats
{
    public class ActyxEvent
    {

        public ActyxEventMetadata Meta { private set; get; }

        public JValue Payload { private set; get; }

        public static Func<EventOnWire, ActyxEvent> From(NodeId nodeId) => ev =>
            new ActyxEvent
            {
                Meta = new ActyxEventMetadata(ev, nodeId),
                Payload = ev.Payload,
            };

        public static IList<ActyxEvent> OrderByEventKey(IList<ActyxEvent> events) =>
            events.OrderBy(x => x.Meta.Lamport).ThenBy(x => x.Meta.Stream).ToList();
    }
}
