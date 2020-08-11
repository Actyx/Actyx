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

	    var s = new EventService();

	    var offsets = await s.offsets();
	    
	    Console.WriteLine(string.Join(Environment.NewLine, offsets));

	    return;
	    
        }
    }
}
