using System;
using System.Threading;
using System.Threading.Tasks;
using Actyx;
using Newtonsoft.Json.Linq;

namespace Sdk.IntegrationTests
{
    class Program2
    {
        static void Main()
        {
            var exitEvent = new ManualResetEvent(false);
            using var client = new WsrpcClient("AAAAX6ZnY3JlYXRlZBsABcXiWPpaPGVhcHBJZHFjb20uZXhhbXBsZS50ZXN0MmZjeWNsZXMFamFwcFZlcnNpb25lMS4wLjBodmFsaWRpdHkaAAFRgGdhcHBNb2RlZXRyaWFsAVHZ5k6MsCsdlinyp4kZ8ahd6lb65k+Hwq8dHTqMeX8DHbKmngnFSq5zZMUOsGo9ggErvQOOZobmgGD3tw526hiyJGKIppEqsgtcJHvAmgnvpBEIFP5T+XmtTUwRA5ORDA==");
            client.Start();
            Task.Run(() =>
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
