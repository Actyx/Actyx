using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Formats
{
    public class ActyxEvent<E>
    {
        public ActyxEventMetadata Meta { internal set; get; }

        public E Payload { internal set; get; }

        public static IList<ActyxEvent<T>> OrderByEventKey<T>(IList<ActyxEvent<T>> events) =>
            events.OrderBy(x => x.Meta.Lamport).ThenBy(x => x.Meta.Stream).ToList();
    }

    internal class MkAxEvt
    {
        public static Func<EventOnWire, ActyxEvent<E>> DeserTyped<E>(NodeId nodeId) => ev =>
            new ActyxEvent<E>
            {
                Meta = new ActyxEventMetadata(ev, nodeId),
                Payload = ev.Payload.ToObject<E>(),
            };

        public static Func<EventOnWire, ActyxEvent<JToken>> From(NodeId nodeId) => ev =>
            new ActyxEvent<JToken>
            {
                Meta = new ActyxEventMetadata(ev, nodeId),
                Payload = ev.Payload,
            };
    }
}
