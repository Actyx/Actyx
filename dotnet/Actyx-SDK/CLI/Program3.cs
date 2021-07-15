using System;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Linq;
using System.Reactive.Threading.Tasks;
using System.Threading;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;
using DeepEqual.Syntax;

namespace CLI
{
    class Program3
    {
        const int N = 100;

        static async Task Main()
        {
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
            using var store = new WebsocketEventStore(wsrpcClient, nodeId);

            List<string> tags = new() { RandomString(8) };

            var persisted =
                Observable
                    .Timer(TimeSpan.Zero, TimeSpan.FromMilliseconds(50))
                    .SelectMany(async _ =>
                        {
                            List<IEventDraft> events = new()
                            {
                                new EventDraft { Tags = tags, Payload = "event" },
                            };
                            PublishResponse response = await store.Publish(events);
                            return response.Data.Zip(events, (metadata, @event) =>
                                new EventOnWire
                                {
                                    Lamport = metadata.Lamport,
                                    Offset = metadata.Offset,
                                    Payload = @event.Payload,
                                    Stream = metadata.Stream,
                                    Tags = @event.Tags,
                                    Timestamp = metadata.Timestamp,
                                    AppId = manifest.AppId,
                                }
                            ).ToObservable();
                        }
                    );
            var subscribe =
                store
                    .Subscribe(null, new Aql($"FROM '{tags[0]}'"))
                    .OfType<EventOnWire>();
            await foreach (var (p, s) in persisted.Zip(subscribe).Take(N).ToAsyncEnumerable())
            {
                p.ShouldDeepEqual(s);
            }
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
