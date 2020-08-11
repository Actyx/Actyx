using System;
using JsonSubTypes;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using System.Net.Http;
using System.Text;
using System.IO;
using System.Net;
using System.Net.Mime;
using System.Threading.Tasks;
using System.Net.Http.Headers;
using System.Collections.Generic;
using System.Threading;

namespace Actyx {

    class StreamingResponse<T> : IAsyncEnumerator<T> {

	private readonly StreamReader reader;

	public StreamingResponse(Stream responseDataStream) {
	    this.reader = new StreamReader(responseDataStream);
	}

	public T Current { get; private set; }

	public async ValueTask<bool> MoveNextAsync() {
	    if (reader.EndOfStream) {
		return false;
	    }

	    string nextLine = await reader.ReadLineAsync();

	    // Empty lines are sent as a means of keep-alive.
	    while (nextLine != "event:event") {
		Console.WriteLine("skipping: " + nextLine);
		nextLine = await reader.ReadLineAsync();
	    }

	    // Immediately after the event:event line we expect the data:{json} line
	    nextLine = await reader.ReadLineAsync();
	    while (!nextLine.StartsWith("data:")) {
		Console.WriteLine("EXPECTED DATA BUT FOUND: " + nextLine);
		nextLine = await reader.ReadLineAsync();
	    }

	    // Drop the "data:" prefix and deserialize
	    string jsonData = nextLine.Substring(5);
	    this.Current = JsonConvert.DeserializeObject<T>(jsonData);

	    return true;
	}

	public async ValueTask DisposeAsync() {
	    reader.Dispose();
	}
    }


    class Request<T> : IAsyncEnumerable<T> {
	private readonly string path;
	private readonly string postData;

	public Request(string path, string postData) {
	    this.path = path;
	    this.postData = postData;
	}

	public IAsyncEnumerator<T> GetAsyncEnumerator(CancellationToken token) {
	    var request = WebRequest.Create(this.path);
	    request.ContentType = "application/json";
	    request.Headers.Add("Authorization", "Bearer AAAARqVnY3JlYXRlZBsABayEzaJD42ZhcHBfaWRoc29tZS5hcHBmY3ljbGVzAGd2ZXJzaW9uZTEuMC4waHZhbGlkaXR5Gv////8Bf1lCGGeTcd1ywvwYue4jEjqTx0LYFTzdBzdyr65FfgYkJSlrbLTNa1R88kJNNa6+t8UDD0F/t8rlEdZAX7vXAcrDkxFVk2QFFi/o9eIlNmk8wd917afsGBD7ap5EOX4M");

	    // Setup POST data:
	    request.Method = "POST";
	    var reqMsgBytes = Encoding.UTF8.GetBytes(this.postData);

	    var dataStream = request.GetRequestStream();
	    dataStream.Write(reqMsgBytes, 0, reqMsgBytes.Length);
	    dataStream.Close();

	    var response = request.GetResponse();

	    return new StreamingResponse<T>(response.GetResponseStream());
	}
    }

    class EventService
    {
	private readonly string endpoint;

	public EventService(string endpoint = "http://localhost:4454/api") {
	    this.endpoint = endpoint;
	}

	public async Task<Dictionary<string, UInt64>> offsets()
	{
	    var request = WebRequest.Create(this.endpoint + "/v2/events/offsets");
	    request.ContentType = "application/json";
	    request.Headers.Add("Authorization", "Bearer AAAARqVnY3JlYXRlZBsABayEzaJD42ZhcHBfaWRoc29tZS5hcHBmY3ljbGVzAGd2ZXJzaW9uZTEuMC4waHZhbGlkaXR5Gv////8Bf1lCGGeTcd1ywvwYue4jEjqTx0LYFTzdBzdyr65FfgYkJSlrbLTNa1R88kJNNa6+t8UDD0F/t8rlEdZAX7vXAcrDkxFVk2QFFi/o9eIlNmk8wd917afsGBD7ap5EOX4M");

	    var response = await request.GetResponseAsync();

	    var reader = new StreamReader(response.GetResponseStream());

	    return JsonConvert.DeserializeObject<Dictionary<string, UInt64>>(reader.ReadLine());
	}
	

	public Request<ISuttMessage> subscribeUntilTimeTravel(string session, string subscription, IDictionary<string, UInt64> offsets)
	{
	    var req = new {
		session,
		subscription,
		offsets
	    };

	    string postData = JsonConvert.SerializeObject(req);

	    return new Request<ISuttMessage>(this.endpoint + "/v2/events/subscribeUntilTimeTravel", postData);
	}


	public Request<ISuttMessage> subscribeUntilTimeTravel(string session, string subscription, params SnapshotCompression[] acceptedFormats)
	{
	    List<string> compression = new List<string>();

	    if (acceptedFormats.Length == 0) {
		compression.Add(SnapshotCompression.None.ToString());
	    } else {
		foreach (var accepted in acceptedFormats) {
		    compression.Add(accepted.ToString().ToLower());
		}
	    }

	    var req = new {
		session,
		subscription,
		snapshot = new {
		    compression
		}
	    };

	    string postData = JsonConvert.SerializeObject(req);
	    Console.WriteLine("posting:" + postData);

	    return new Request<ISuttMessage>(this.endpoint + "/v2/events/subscribeUntilTimeTravel", postData);
	}

	public Request<Event> subscribe()
	{
	    var req = new {
		subscriptions = new List<object>() {
		    new {
			semantics = "whatever",
		    },
		}
	    };

	    // string postData = "{\"subscriptions\": [{}]}";
	    string postData = JsonConvert.SerializeObject(req);

	    return new Request<Event>(this.endpoint + "/v2/events/subscribe", postData);
	}
    }
}
