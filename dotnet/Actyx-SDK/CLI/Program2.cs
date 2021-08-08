using System;
using System.Threading;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Utils;
using Actyx.Sdk.Wsrpc;
using Newtonsoft.Json.Linq;

namespace Actyx.CLI
{
    class Program2
    {
        static async Task Main()
        {
            var exitEvent = new ManualResetEvent(false);
            var serializer = EventStoreSerializer.Create();
            var converter = new JsonContentConverter(serializer);

            var manifest = new AppManifest()
            {
                AppId = "com.example.ax-ws-client-tests",
                DisplayName = "ax ws client tests",
                Version = typeof(Program).Assembly.GetName().Version.ToString(),
            };
            var axHttpClient = await AxHttpClient.Create("http://localhost:4454/api/v2/", manifest, converter);
            Uri axWs = new($"ws://localhost:4454/api/v2/events?{axHttpClient.Token}");
            using var client = new WsrpcClient(axWs);
            client.Start();
            var _ = Task.Run(() =>
            {
                var request = JToken.Parse(@"{ ""query"": ""FROM 'com.actyx.1'"", ""order"": ""asc""}");
                client
                    .Request("subscribe", request)
                    .Subscribe(
                        next => Console.WriteLine($">>> next: {next}"),
                        error => Console.WriteLine($">>> error: {error}")
                    );
            });

            exitEvent.WaitOne();
        }
    }
}
