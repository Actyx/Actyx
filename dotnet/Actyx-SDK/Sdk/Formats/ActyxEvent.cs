using System;
using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;

namespace Actyx.Sdk.Formats
{
    public class ActyxEvent<E> : IComparable<ActyxEvent<E>>
    {
        public ActyxEventMetadata Meta { internal set; get; }

        public E Payload { internal set; get; }

        public static IList<ActyxEvent<T>> OrderByEventKey<T>(IList<ActyxEvent<T>> events) =>
            events.OrderBy(x => x.Meta.Lamport).ThenBy(x => x.Meta.Stream).ToList();

        public int CompareTo(ActyxEvent<E> other)
        {
            // If other is not a valid object reference, this instance is greater.
            if (other == null) return 1;

            return Meta.CompareTo(other.Meta);
        }

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
