using System.Collections.Generic;

namespace Actyx
{
    // Select events of type E -- any such selection may be converted to a more general selection,
    // to be consumed by code that can handle a more general type.
    public interface IFrom<out E> : IEventSelection
    {
        List<RawTagSet> UnderlyingSets { get; }
    }
}
