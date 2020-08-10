using System;
using System.Threading.Tasks;

namespace Actyx
{
    class Program
    {
        static async Task Main(string[] args)
        {
            Console.WriteLine("Hello World!");
	    await foreach (string line in new EventService().subscribe2()) {
		Console.WriteLine("ffffff");
		Console.WriteLine(line);
	    }
        }
    }
}
