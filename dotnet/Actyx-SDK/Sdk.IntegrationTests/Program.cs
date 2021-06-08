using Actyx;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Sdk.IntegrationTests
{
    class Program
    {
        static async Task Main(string[] args)
        {
            var s = await EventService.ForApp("some.app");

            IEventDraft evt = new EventDraft
            {
                Tags = new List<string>() { "test0", "test1" },

                Payload = "my-test-payload-hello :)"
            };

            var p = await s.Publish(new List<IEventDraft>() { evt });

            Console.WriteLine(p.ToString());

            var offsets = await s.Offsets();

            string query = "'test0' & 'test1'";

            var a = s.QueryStreaming(query, offsets, EventsOrder.LamportReverse);

            // var t = s.subscribeMonotonic("foo", query, SnapshotCompression.None);

            await foreach (var q in a)
            {
                // await foreach (var q in new EventService().subscribe()) {
                Console.WriteLine("ffffff");
                Console.WriteLine(q.Type);

                if (q is Event)
                {
                    Console.WriteLine((q as Event).Payload);
                    Console.WriteLine((q as Event).CaughtUp);
                }
            }
        }
    }
}
