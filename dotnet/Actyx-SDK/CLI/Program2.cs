using System;
using System.Threading;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.Formats;
using Actyx.Sdk.Wsrpc;
using Newtonsoft.Json.Linq;

namespace Actyx.CLI
{
    class Program2
    {
        static void Main()
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
            var apiUri = new Uri("http://localhost:4454/api/v2/");
            var httpClient = new AuthenticatedClient(manifest, new Uri(apiUri, "events/"), new Uri(apiUri, "auth"), converter);
            var token = httpClient.GetToken();

            using var client = new WsrpcClient(new Uri($"ws://localhost:4454/api/v2/events?{token}"));
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
