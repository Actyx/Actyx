using Actyx;
using System;
using System.Threading.Tasks;

namespace cs_sdk.Tests
{
    class Program
    {
        static async Task Main(string[] args)
        {
	    var s = new EventService();
	    string query = "'semantics:edge.ax.sf.UiSession'";

	    var t = s.subscribeUntilTimeTravel("foo", query, SnapshotCompression.None);

	    await foreach (var q in t) {
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
