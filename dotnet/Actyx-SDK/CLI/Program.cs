using System;
using System.Collections.Generic;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.CommandLine.Parsing;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Sdk.Formats;
using Newtonsoft.Json;

namespace Actyx.CLI
{
    class Program
    {
        private static async Task<IEventStore> MkStore(AppManifest manifest, bool websocket, string node)
        {
            var opts = new ActyxOpts()
            {
                Transport = websocket ? Transport.WebSocket : Transport.Http,
            };
            if (!string.IsNullOrWhiteSpace(node))
            {
                var hostPort = node.Split(":");
                switch (hostPort.Length)
                {
                    case 1:
                        opts.Host = hostPort[0];
                        break;
                    case 2:
                        opts.Host = hostPort[0];
                        opts.Port = Convert.ToUInt32(hostPort[1]);
                        break;
                }
            }
            return await EventStore.Create(manifest, opts);
        }

        private static OffsetMap ParseBounds(ArgumentResult res) =>
            EventStore.Protocol.Deserialize<OffsetMap>(res.Tokens[0].Value);

        private static AppManifest ParseManifest(ArgumentResult res)
        {
            if (res.Tokens.Count == 0)
            {
                return new()
                {
                    AppId = "com.example.actyx-cli",
                    DisplayName = "Actyx .NET CLI",
                    Version = typeof(Program).Assembly.GetName().Version.ToString(),
                };
            }
            return EventStore.Protocol.Deserialize<AppManifest>(res.Tokens[0].Value);
        }

        private static Action<T> Serializer<T>()
        {
            var serializer = EventStoreSerializer.Create(pretty: false);
            return t =>
            {
                serializer.Serialize(Console.Out, t);
                Console.Out.WriteLine();
            };
        }

        private static Command Query()
        {
            var cmd = new Command("query"){
                new Option<EventsOrder>("--order"){ IsRequired = true },
                new Option<OffsetMap>("--lower-bound", ParseBounds),
                new Option<OffsetMap>("--upper-bound", ParseBounds),
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, OffsetMap, Aql, EventsOrder>(async (manifest, websocket, node, lowerBound, upperBound, query, order) =>
            {
                using var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .Query(lowerBound, upperBound, query, order)
                    .ForEachAsync(Serializer<IResponseMessage>());
            });
            return cmd;
        }

        private static Command Subscribe()
        {
            var cmd = new Command("subscribe"){
                new Option<OffsetMap>("--lower-bound", ParseBounds),
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, Aql>(async (manifest, websocket, node, lowerBound, query) =>
            {
                using var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .Subscribe(lowerBound, query)
                    .ForEachAsync(Serializer<IResponseMessage>());
            });
            return cmd;
        }


        private static Command SubscribeMonotonic()
        {
            var cmd = new Command("subscribe_monotonic"){
                new Option<string>("--session"){ IsRequired = true, Arity = ArgumentArity.ExactlyOne },
                new Option<OffsetMap>("--lower-bound", ParseBounds){ IsRequired = true, Arity = ArgumentArity.ExactlyOne },
                new Argument<string>("node"),
                new Argument<Aql>("query", res => new Aql(res.Tokens[0].Value)){ Arity = ArgumentArity.ExactlyOne },
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, OffsetMap, string, Aql>(async (manifest, websocket, node, lowerBound, session, query) =>
            {
                using var eventStore = await MkStore(manifest, websocket, node);
                await eventStore
                    .SubscribeMonotonic(session, lowerBound, query)
                    .ForEachAsync(Serializer<ISubscribeMonotonicResponse>());
            });
            return cmd;
        }

        private static Command Offsets()
        {
            var cmd = new Command("offsets")
            {
                new Argument<string>("node"),
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string>(async (manifest, websocket, node) =>
            {
                using var eventStore = await MkStore(manifest, websocket, node);
                var offsets = await eventStore.Offsets();
                Serializer<OffsetsResponse>()(offsets);
            });
            return cmd;
        }

        private static Command Publish()
        {
            var serializer = EventStoreSerializer.Create();
            var cmd = new Command("publish"){
                new Argument<string>("node"),
                new Argument<IEnumerable<EventDraft>>("events", (ArgumentResult res) =>
                    res.Tokens
                        .Select(t => {
                            using var reader = new System.IO.StringReader(t.Value);
                            using var jsonReader = new JsonTextReader(reader);
                            return serializer.Deserialize<EventDraft>(jsonReader);
                        })
                        .ToArray()
                ),
            };
            cmd.Handler = CommandHandler.Create<AppManifest, bool, string, IEnumerable<EventDraft>>(async (manifest, websocket, node, events) =>
            {
                using var eventStore = await MkStore(manifest, websocket, node);
                var response = await eventStore.Publish(events.Cast<IEventDraft>());
                Serializer<PublishResponse>()(response);
            });
            return cmd;
        }

        static async Task<int> Main(string[] args)
        {
            var events = new Command("events"){
                Offsets(),
                Query(),
                Subscribe(),
                SubscribeMonotonic(),
                Publish(),
            };
            events.AddGlobalOption(new Option<bool>(new string[] { "--websocket", "-ws" }));
            events.AddGlobalOption(new Option<AppManifest>(new string[] { "--manifest", "-m" }, ParseManifest, isDefault: true) { Arity = ArgumentArity.ZeroOrOne });
            var rootCmd = new RootCommand() { events };
            return await rootCmd.InvokeAsync(args);
        }
    }
}
