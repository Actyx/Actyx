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

#nullable enable
    class AsyncResponse<T> : IAsyncEnumerator<T> where T : class{

	private readonly StreamReader reader;

	public AsyncResponse(Stream responseDataStream) {
	    this.reader = new StreamReader(responseDataStream);
	}

	private T? current = null;

	public T? Current {
	    get {
		return this.current;
	    }
	}
#nullable disable

	public async ValueTask<bool> MoveNextAsync() {
	    if (reader.EndOfStream) {
		return false;
	    }

	    var nextLine = await reader.ReadLineAsync();

	    if (!String.IsNullOrEmpty(nextLine)) {
		this.current = JsonConvert.DeserializeObject<T>(nextLine);
	    } else {
		Console.WriteLine("empty line");
	    }

	    return true;
	}

	public async ValueTask DisposeAsync() {
	    reader.Dispose();
	}
    }


    class DelayedResponse<T> : IAsyncEnumerable<T> where T : class {
	private readonly string path;
	private readonly string postData;

	public DelayedResponse(string path, string postData) {
	    this.path = path;
	    this.postData = postData;
	}

	public IAsyncEnumerator<T> GetAsyncEnumerator(CancellationToken token) {
	    var request = WebRequest.Create(this.path);
	    request.Method = "POST";
	    request.ContentType = "application/json";
	    request.Headers.Add("Authorization", "AAAARqVnY3JlYXRlZBsABayEzaJD42ZhcHBfaWRoc29tZS5hcHBmY3ljbGVzAGd2ZXJzaW9uZTEuMC4waHZhbGlkaXR5Gv////8Bf1lCGGeTcd1ywvwYue4jEjqTx0LYFTzdBzdyr65FfgYkJSlrbLTNa1R88kJNNa6+t8UDD0F/t8rlEdZAX7vXAcrDkxFVk2QFFi/o9eIlNmk8wd917afsGBD7ap5EOX4M");

	    var reqMsgBytes = Encoding.UTF8.GetBytes(this.postData);

	    var dataStream = request.GetRequestStream();
	    // Write the data to the request stream.
	    dataStream.Write(reqMsgBytes, 0, reqMsgBytes.Length);
	    // Close the Stream object.
	    dataStream.Close();

	    var response = request.GetResponse();

	    return new AsyncResponse<T>(response.GetResponseStream());
	}
    }

    class EventService
    {
	private readonly string endpoint;

	public EventService(string endpoint = "http://localhost:4454/api") {
	    this.endpoint = endpoint;
	}

	public DelayedResponse<Event> subscribeUntilTimeTravel(string session, string subscription, IDictionary<string, UInt64> offsets) {
	    var req = new {
		session,
		subscription,
		offsets
	    };

	    string postData = JsonConvert.SerializeObject(req);

	    return new DelayedResponse<Event>(this.endpoint + "/v2/events/subscribeUntilTimeTravel", postData);
	}

	public DelayedResponse<EventV1> subscribe()
	{
	    return new DelayedResponse<EventV1>(this.endpoint + "/v1/events/subscribe", "{\"subscriptions\": [{}]}");
	}
    }
}
