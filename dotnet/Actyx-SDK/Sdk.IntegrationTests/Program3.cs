using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Formats;

namespace Sdk.IntegrationTests
{
    class Program3
    {
        static async Task Main()
        {
            var exitEvent = new ManualResetEvent(false);

            var manifest = new AppManifest()
            {
                AppId = "com.example.ax-ws-client-tests",
                DisplayName = "ax ws client tests",
                Version = "1.0.0"
            };
            var baseUri = "http://localhost:4454/api/v2/";
            var nodeId = (await AxHttpClient.Create(baseUri, manifest)).NodeId;
            var token = (await AxHttpClient.GetToken(new Uri(baseUri), manifest)).Token;
            var wsrpcClient = new WsrpcClient(new Uri($"ws://localhost:4454/api/v2/events?{token}"));
            using var store = new WebsocketEventStore(wsrpcClient, "com.example.ax-ws-client-tests", nodeId);

            var _ = Task.Run(async () =>
            {
                var persist = Observable
                    .Timer(TimeSpan.Zero, TimeSpan.FromSeconds(5))
                    .Select(x => store.PersistEvents(new List<IEventDraft>() {
                        new EventDraft { Tags = new List<string>() { "com.actyx.1" }, Payload = "live_event" },
                }));
                // await persist.ForEachAsync(e => Console.WriteLine($"persisted: ${e}"));
                await foreach (var p in persist.ToAsyncEnumerable())
                {
                    Console.WriteLine($"persisted: ${p}");
                }
            });

            var subscribe = store.Subscribe(null, new Aql("FROM 'com.actyx.1'"));
            // await subscribe.ForEachAsync(e => Console.WriteLine($"subscribed: {e}"));
            await foreach (var s in subscribe.ToAsyncEnumerable())
            {
                Console.WriteLine($"subscribed: {s}");
            }

            exitEvent.WaitOne();
        }
    }
}
