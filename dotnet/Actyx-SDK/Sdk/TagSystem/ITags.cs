using System.Collections.Generic;

namespace Actyx
{
    // A list of tags to attach to events.
    // Payloads are allowed to be of a more specific type than E.
    public interface ITags<in E>
    {
        // ..
        RawTagSet Underlying { get; }

        public IEventDraft Apply(E eventData);

        public List<IEventDraft> Apply(params E[] events);
    }
}
