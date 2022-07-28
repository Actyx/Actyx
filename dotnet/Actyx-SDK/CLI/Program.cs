using System;
using System.Collections.Generic;
using System.CommandLine;
using System.CommandLine.Invocation;
using System.CommandLine.Parsing;
using System.Linq;
using System.Reactive.Linq;
using System.Threading.Tasks;
using Actyx.Documents.Driver;
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
            var am = new Command("am")
            {
                Handler = CommandHandler.Create(() =>
                {
                    var (docalloc, doc) = Value(automerge.AMcreate(), AMvalue_Tag.A_MVALUE_DOC, x => x.doc);

                    var (initialalloc, initial) = Value(automerge.AMgetHeads(doc), AMvalue_Tag.A_MVALUE_CHANGE_HASHES, x => x.change_hashes);

                    Check(automerge.AMmapPutBool(doc, automerge.ROOT, "done", true));
                    var (pointalloc, point) = Value(automerge.AMmapPutObject(doc, automerge.ROOT, "p", AMobjType.A_MOBJ_TYPE_MAP), AMvalue_Tag.A_MVALUE_OBJ_ID, x => x.obj_id);
                    Check(automerge.AMmapPutInt(doc, point, "x", 23));
                    Check(automerge.AMmapPutInt(doc, point, "y", 42));
                    Check(automerge.AMsave(doc));
                    automerge.AMfree(pointalloc);

                    var (changealloc, changes) = Value(automerge.AMgetChanges(doc, initial), AMvalue_Tag.A_MVALUE_CHANGES, x => x.changes);
                    var c_len = automerge.AMchangesSize(changes);
                    Console.WriteLine($"changes: {c_len}");
                    var c = new List<byte[]>((int)c_len);
                    for (var p = automerge.AMchangesNext(changes, 1); !automerge.AMchangeIsEmpty(p); p = automerge.AMchangesNext(changes, 1))
                    {
                        Console.WriteLine("--");
                        Console.WriteLine($"{BitConverter.ToString(Bytes.FromSpan(automerge.AMchangeRawBytes(p)))}");
                    }

                    var (keys1alloc, keys1) = Value(automerge.AMkeys(doc, automerge.ROOT, automerge.NOW), AMvalue_Tag.A_MVALUE_STRINGS, x => x.strings);
                    for (var count = automerge.AMstringsSize(keys1); count > 0; --count)
                    {
                        var name = automerge.AMstringsNext(keys1, 1);
                        var kind = Check(automerge.AMmapGet(doc, automerge.ROOT, name));
                        Console.WriteLine($" - {name}: {kind}");
                    }
                    automerge.AMfree(keys1alloc);

                    automerge.AMfree(initialalloc);
                    automerge.AMfree(docalloc);

                    Console.WriteLine("restore");

                    var (docalloc2, doc2) = Value(automerge.AMcreate(), AMvalue_Tag.A_MVALUE_DOC, x => x.doc);
                    Check(automerge.AMapplyChanges(doc2, automerge.AMresultValue(changealloc).changes));

                    var (keys2alloc, keys2) = Value(automerge.AMkeys(doc2, automerge.ROOT, automerge.NOW), AMvalue_Tag.A_MVALUE_STRINGS, x => x.strings);
                    for (var count = automerge.AMstringsSize(keys2); count > 0; --count)
                    {
                        var name = automerge.AMstringsNext(keys2, 1);
                        var kind = Check(automerge.AMmapGet(doc2, automerge.ROOT, name));
                        Console.WriteLine($" - {name}: {kind}");
                    }
                    automerge.AMfree(keys2alloc);

                    var done = ValueFree(automerge.AMmapGet(doc2, automerge.ROOT, "done"), AMvalue_Tag.A_MVALUE_BOOLEAN, x => x.boolean);
                    Console.WriteLine($"done={done}");
                    var (objalloc, obj) = Value(automerge.AMmapGet(doc2, automerge.ROOT, "p"), AMvalue_Tag.A_MVALUE_OBJ_ID, x => x.obj_id);
                    var x = ValueFree(automerge.AMmapGet(doc2, obj, "x"), AMvalue_Tag.A_MVALUE_INT, x => x.int_);
                    var y = ValueFree(automerge.AMmapGet(doc2, obj, "y"), AMvalue_Tag.A_MVALUE_INT, x => x.int_);
                    Console.WriteLine($"x={x} y={y}");
                    automerge.AMfree(objalloc);
                    automerge.AMfree(docalloc2);
                })
            };
            var rootCmd = new RootCommand(){
                events,
                am
            };
            return await rootCmd.InvokeAsync(args);
        }

        private static AMvalue_Tag Check(SWIGTYPE_p_AMresult result)
        {
            if (automerge.AMresultStatus(result) != AMstatus.A_MSTATUS_OK)
            {
                throw new Exception(automerge.AMerrorMessage(result));
            }
            var ret = automerge.AMresultValue(result).tag;
            automerge.AMfree(result);
            return ret;
        }
        private static (SWIGTYPE_p_AMresult, T) Value<T>(SWIGTYPE_p_AMresult result, AMvalue_Tag tag, Func<AMvalue, T> f)
        {
            if (automerge.AMresultStatus(result) != AMstatus.A_MSTATUS_OK)
            {
                throw new Exception($"automerge: {automerge.AMerrorMessage(result)}");
            }
            var value = automerge.AMresultValue(result);
            if (value.tag != tag)
            {
                throw new Exception($"wrong tag: expected {tag} got {value.tag}");
            }
            var ret = f(value);
            return (result, ret);
        }
        private static T ValueFree<T>(SWIGTYPE_p_AMresult result, AMvalue_Tag tag, Func<AMvalue, T> f)
        {
            var (alloc, res) = Value(result, tag, f);
            automerge.AMfree(alloc);
            return res;
        }
    }
}

namespace Actyx.Documents.Driver
{
    public static class Bytes
    {
        public static byte[] FromSpan(AMbyteSpan span)
        {
            var len = span.count;
            var arr = new byte[len];
            for (int i = 0; i < len; ++i)
            {
                arr[i] = automerge.bytes_getitem(span.src, i);
            }
            return arr;
        }
    }
}
