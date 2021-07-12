using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Threading;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
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

            var tag = RandomString(8);

            var _ = Task.Run(async () =>
            {
                var persist = Observable
                    .Timer(TimeSpan.Zero, TimeSpan.FromSeconds(5))
                    .SelectMany(x => Observable.FromAsync(() => store.Publish(new List<IEventDraft>() {
                        new EventDraft { Tags = new List<string>() { tag }, Payload = "live_event" },
                    })));
                await foreach (var batch in persist.ToAsyncEnumerable())
                {
                    foreach (var p in batch)
                    {
                        Console.WriteLine($"persisted: {Proto<EventOnWire>.Serialize(p)}");
                    }
                }
            });

            var subscribe = store.Subscribe(null, new Aql($"FROM '{tag}'"));
            await foreach (var s in subscribe.ToAsyncEnumerable())
            {
                Console.WriteLine($"subscribed: {Proto<IEventOnWire>.Serialize(s)}");
            }

            exitEvent.WaitOne();
        }

        private static readonly Random random = new();
        public static string RandomString(int length)
        {
            const string chars = "abcdefghijklmnopqrstuvwxyz";
            return new string(Enumerable.Repeat(chars, length)
              .Select(s => s[random.Next(s.Length)]).ToArray());
        }
    }
}
