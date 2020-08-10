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
    
    class AsyncResponse : IAsyncEnumerator<string> {

	private readonly StreamReader reader;

	public AsyncResponse(Stream responseDataStream) {
	    this.reader = new StreamReader(responseDataStream);
	}

#nullable enable
	private string? current = null;
	
	public string? Current {
	    get {
		return this.current;
	    }
	}
#nullable disable

	public async ValueTask<bool> MoveNextAsync() {
	    if (reader.EndOfStream) {
		return false;
	    }

	    var next = await reader.ReadLineAsync();

	    this.current = next;
	    return true;
	}

	public async ValueTask DisposeAsync() {
	    reader.Dispose();
	}
    }

    
    class DelayedResponse : IAsyncEnumerable<string> {
	private readonly string path;
	private readonly string postData;
	
	public DelayedResponse(string path, string postData) {
	    this.path = path;
	    this.postData = postData;
	}

	public IAsyncEnumerator<string> GetAsyncEnumerator(CancellationToken token) {
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

	    return new AsyncResponse(response.GetResponseStream());
	}
    }

    class EventService
    {

	private readonly string endpoint;
	
	public EventService(string endpoint = "http://localhost:4454/api") {
	    this.endpoint = endpoint;
	}
	
	public DelayedResponse subscribeUntilTimeTravel(string session, string subscription, IDictionary<string, UInt64> offsets) {
	    var req = new {
		session,
		subscription,
		offsets
	    };
	    
	    string postData = JsonConvert.SerializeObject(req);
	    
	    return new DelayedResponse(this.endpoint + "/v2/events/subscribeUntilTimeTravel", postData);
	}

	public void subscribe() {
	    
	    var request = WebRequest.Create(this.endpoint + "/v1/events/subscribe");
	    request.Method = "POST";
	    request.ContentType = "application/json";

	    var reqMsgBytes = Encoding.UTF8.GetBytes("{\"subscriptions\": [{}]}");
	    
	    var dataStream = request.GetRequestStream();
	    // Write the data to the request stream.
	    dataStream.Write(reqMsgBytes, 0, reqMsgBytes.Length);
	    // Close the Stream object.
	    dataStream.Close();

	    var response = request.GetResponse();

	    using (dataStream = response.GetResponseStream()) {
		// Open the stream using a StreamReader for easy access.
		var reader = new StreamReader(dataStream);

		// Read the content.
		while (!reader.EndOfStream) {
		    string responseFromServer = reader.ReadLine();

		    // Display the content.
		    Console.WriteLine($"Line: {responseFromServer}");
		}
	    }


	    // Close the response.
	    response.Close();
	}
    }
}
