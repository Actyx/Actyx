using System;
using System.Threading;
using System.Threading.Tasks;
using Actyx.Sdk.AxHttpClient;
using Actyx.Sdk.AxWebsocketClient;
using Newtonsoft.Json.Linq;

namespace CLI
{
    class Program2
    {
        static async Task Main()
        {
            var exitEvent = new ManualResetEvent(false);
            var token = (await AxHttpClient.GetToken(new Uri("http://localhost:4454/api/v2/"), new()
            {
                AppId = "com.example.ax-ws-client-tests",
                DisplayName = "ax ws client tests",
                Version = "1.0.0"
            })).Token;
            var uri = new Uri($"http://localhost:4454/api/v2/events?{token}");
            using var client = new WsrpcClient(uri);
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
