using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Wsrpc;
using DeepEqual.Syntax;
using Newtonsoft.Json;

namespace Actyx.CLI
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
                Version = typeof(Program).Assembly.GetName().Version.ToString(),
            };
            var serializer = EventStoreSerializer.Create();
            var converter = new JsonContentConverter(serializer);
            var apiUri = new Uri("http://localhost:4454/api/v2/");
            var httpClient = new AuthenticatedClient(manifest, new Uri(apiUri, "events/"), new Uri(apiUri, "auth"), converter);
            var token = await httpClient.GetToken();

            var nodeIdReq = new HttpRequestMessage(HttpMethod.Get, new Uri(apiUri, "node/id"));
            nodeIdReq.Headers.Add("Accept", "application/json");
            nodeIdReq.Content = converter.ToContent(manifest);
            var nodeIdResp = await httpClient.Fetch(nodeIdReq);
            var nodeId = new NodeId(await nodeIdResp.Content.ReadAsStringAsync());

            using var wsrpcClient = new WsrpcClient(new Uri($"ws://localhost:4454/api/v2/events?{token}"));
            using var store = new WebsocketEventStore(wsrpcClient, nodeId, manifest.AppId);

            List<string> tags = new() { AxRandom.String(8) };

            var persisted =
                Observable
                    .Timer(TimeSpan.Zero, TimeSpan.FromMilliseconds(50))
                    .SelectMany(_ =>
                    {
                        List<IEventDraft> events = new()
                        {
                            new EventDraft { Tags = tags, Payload = AxRandom.String(20) },
                        };
                        return Observable
                            .FromAsync(() => store.Publish(events))
                            .SelectMany(response => response.Data.Zip(events, (metadata, @event) =>
                                    new EventOnWire
                                    {
                                        Lamport = metadata.Lamport,
                                        Offset = metadata.Offset,
                                        Payload = JsonConvert.SerializeObject(@event.Payload),
                                        Stream = metadata.Stream,
                                        Tags = @event.Tags,
                                        Timestamp = metadata.Timestamp,
                                        AppId = manifest.AppId,
                                    }
                            ));
                    });

            var subscribed =
                store
                    .Subscribe(null, new Aql($"FROM '{tags[0]}'"))
                    .OfType<EventOnWire>();

            await foreach (var (p, s) in persisted.Zip(subscribed).Take(N).ToAsyncEnumerable())
            {
                p.ShouldDeepEqual(s);
            }
        }
    }
}
