using System;
using System.Threading.Tasks;
using System.Collections.Generic;

namespace Actyx
{
    class Program
    {
        static async Task Main(string[] args)
        {
            Console.WriteLine("Hello World!");
	    string query = "'semantics:edge.ax.sf.UiSession'";

	    // await foreach (string line in new EventService().subscribeUntilTimeTravel("foo", query, new Dictionary<string, UInt64>())) {
	    await foreach (var q in new EventService().subscribe()) {
	    	Console.WriteLine("ffffff");
	    	Console.WriteLine(q.payload);
	    }
        }
    }
}
