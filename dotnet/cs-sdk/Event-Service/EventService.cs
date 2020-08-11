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

	    var nextLine = await reader.ReadLineAsync();

	    if (!String.IsNullOrEmpty(nextLine)) {
		this.Current = JsonConvert.DeserializeObject<T>(nextLine);
	    } else {
		Console.WriteLine("empty line");
	    }

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
	    request.Method = "POST";
	    request.ContentType = "application/json";
	    request.Headers.Add("Authorization", "AAAARqVnY3JlYXRlZBsABayEzaJD42ZhcHBfaWRoc29tZS5hcHBmY3ljbGVzAGd2ZXJzaW9uZTEuMC4waHZhbGlkaXR5Gv////8Bf1lCGGeTcd1ywvwYue4jEjqTx0LYFTzdBzdyr65FfgYkJSlrbLTNa1R88kJNNa6+t8UDD0F/t8rlEdZAX7vXAcrDkxFVk2QFFi/o9eIlNmk8wd917afsGBD7ap5EOX4M");

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

	public Request<Event> subscribeUntilTimeTravel(string session, string subscription, IDictionary<string, UInt64> offsets) {
	    var req = new {
		session,
		subscription,
		offsets
	    };

	    string postData = JsonConvert.SerializeObject(req);

	    return new Request<Event>(this.endpoint + "/v2/events/subscribeUntilTimeTravel", postData);
	}

	public Request<EventV1> subscribe()
	{
	    return new Request<EventV1>(this.endpoint + "/v1/events/subscribe", "{\"subscriptions\": [{}]}");
	}
    }
}
