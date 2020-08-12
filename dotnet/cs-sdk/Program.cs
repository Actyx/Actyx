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

	    var s = new EventService("Bearer AAAARqVnY3JlYXRlZBsABayEzaJD42ZhcHBfaWRoc29tZS5hcHBmY3ljbGVzAGd2ZXJzaW9uZTEuMC4waHZhbGlkaXR5Gv////8Bf1lCGGeTcd1ywvwYue4jEjqTx0LYFTzdBzdyr65FfgYkJSlrbLTNa1R88kJNNa6+t8UDD0F/t8rlEdZAX7vXAcrDkxFVk2QFFi/o9eIlNmk8wd917afsGBD7ap5EOX4M");

	    var offsets = await s.offsets();
	    
	    Console.WriteLine(string.Join(Environment.NewLine, offsets));

	    return;
	    
        }
    }
}
