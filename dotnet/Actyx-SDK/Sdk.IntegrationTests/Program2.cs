using System;
using System.Threading;
using System.Threading.Tasks;
using Actyx;
using Newtonsoft.Json.Linq;

namespace Sdk.IntegrationTests
{
    class Program2
    {
        static async Task Main(string[] args)
        {
            var exitEvent = new ManualResetEvent(false);
            using (var client = new WsrpcClient("AAAAX6ZnY3JlYXRlZBsABcVGv8E43mVhcHBJZHFjb20uZXhhbXBsZS50ZXN0MmZjeWNsZXMBamFwcFZlcnNpb25lMS4wLjBodmFsaWRpdHkaAAFRgGdhcHBNb2RlZXRyaWFsAVHZ5k6MsCsdlinyp4kZ8ahd6lb65k+Hwq8dHTqMeX8DFg6zpqmlKciMoYDF6v0TmtmG10qZiCPs2qwIV5IxfXW4J60l74H/pzCShzVjKnKZHz80uoHOUdvE69/6tzzbBw=="))
            {
                client.Start();
                _ = Task.Run(() => client.Request("subscribe", JObject.Parse(@"{ ""query"": ""FROM allEvents"", ""order"": ""asc""}"))
                .Subscribe(x => Console.WriteLine($">>> {x}")));

                exitEvent.WaitOne();
            }
        }
    }
}
