using System;
using System.Collections.Generic;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.CommandLine.Parsing;
using System.Linq;
using System.Threading.Tasks;
using Actyx;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Actyx.Sdk.Formats;

namespace Sdk.IntegrationTests
{
    class CLI
    {
        private static const Uri Authority = "localhost:4454";

        private static async Task<IEventStore> MkStore(bool websocket)
        {
            AppManifest manifest = new()
            {
                AppId = "com.example.actyx-cli",
                DisplayName = "Actyx .NET CLI",
                Version = "0.0.1"
            };
            if (websocket)
            {
                var baseUri = $"http://{Authority}/api/v2/";
                var nodeId = (await AxHttpClient.Create(baseUri, manifest)).NodeId;
                var token = (await AxHttpClient.GetToken(new Uri(baseUri), manifest)).Token;
                var wsrpcClient = new WsrpcClient(new Uri($"ws://{Authority}/api/v2/events?{token}"));
                return new WebsocketEventStore(wsrpcClient, manifest.AppId, nodeId);
            }
            else
            {
                var httpClient = await AxHttpClient.Create($"http://{Authority}", manifest);
                return new HttpEventStore(httpClient);
            }
        }

        private static OffsetMap ParseBounds(ArgumentResult res) =>
            Proto<OffsetMap>.Deserialize(res.Tokens[0].Value);

        private static Command Query()
        {
            var cmd = new Command("query"){
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
                new Option<EventsOrder>("--order"){ Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--lower-bound", ParseBounds){ Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--upper-bound", ParseBounds){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<bool, OffsetMap, OffsetMap, Aql, EventsOrder>(async (websocket, lowerBound, upperBound, query, order) =>
            {
                var eventStore = await MkStore(websocket);
                await foreach (var e in eventStore.Query(lowerBound, upperBound, query, order).ToAsyncEnumerable())
                {
                    Console.WriteLine(Proto<IEventOnWire>.Serialize(e));
                }
            });
            return cmd;
        }

        private static Command Subscribe()
        {
            var cmd = new Command("subscribe"){
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
                new Option<EventsOrder>("--order"){ Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--lower-bound", ParseBounds){ Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--upper-bound", ParseBounds){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<bool, OffsetMap, OffsetMap, Aql, EventsOrder>(async (websocket, lowerBound, upperBound, query, order) =>
            {
                var eventStore = await MkStore(websocket);
                await foreach (var e in eventStore.Query(lowerBound, upperBound, query, order).ToAsyncEnumerable())
                {
                    Console.WriteLine(Proto<IEventOnWire>.Serialize(e));
                }
            });
            return cmd;
        }

        private static Command Offsets() =>
            new Command("offsets")
            {
                Handler = CommandHandler.Create<bool>(async (websocket) =>
                {
                    var eventStore = await MkStore(websocket);
                    var offsets = await eventStore.Offsets();
                    Console.WriteLine(Proto<OffsetsResponse>.Serialize(offsets));
                })
            };

        private static Command Publish()
        {
            var cmd = new Command("publish"){
                new Argument<IEnumerable<EventDraft>>("events", (ArgumentResult res) =>
                    res.Tokens.Select(t => Proto<EventDraft>.Deserialize(t.Value)).ToArray()
                )
            };
            cmd.Handler = CommandHandler.Create<bool, IEnumerable<EventDraft>>(async (websocket, events) =>
            {
                var eventStore = await MkStore(websocket);
                foreach (var res in await eventStore.Publish(events.Cast<IEventDraft>()))
                {
                    Console.WriteLine(Proto<IEventOnWire>.Serialize(res));
                }
            });
            return cmd;
        }

        static async Task<int> Main(string[] args)
        {
            var events = new Command("events"){
                Offsets(),
                Query(),
                Subscribe(),
                Publish(),
            };
            events.AddGlobalOption(new Option<bool>(new string[] { "--websocket", "-ws" }));
            var rootCmd = new RootCommand() { events };
            return await rootCmd.InvokeAsync(args);
        }
    }
}
