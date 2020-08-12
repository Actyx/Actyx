using Actyx;
using System;
using System.Threading.Tasks;

namespace cs_sdk.Tests
{
    class Program
    {
        static async Task Main(string[] args)
        {
	    var s = await EventService.ForApp("some.app");

	    var offsets = await s.Offsets();
	    
	    string query = "'semantics:edge.ax.sf.UiSession'";

	    var a = s.Query(query, offsets, EventsOrder.LamportReverse);

	    // var t = s.subscribeUntilTimeTravel("foo", query, SnapshotCompression.None);

	    await foreach (var q in a) {
	    // await foreach (var q in new EventService().subscribe()) {
	    	Console.WriteLine("ffffff");
	    	Console.WriteLine(q.Type);

	    	if (q is Event) {
	    	    Console.WriteLine((q as Event).Payload);
	    	    Console.WriteLine((q as Event).CaughtUp);
	    	}
	    }
        }
    }
}
