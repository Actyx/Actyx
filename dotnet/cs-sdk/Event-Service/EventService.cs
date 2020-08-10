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


namespace Actyx {

    class EventService
    {

	private readonly string endpoint;
	
	public EventService(string endpoint = "http://localhost:4454/api") {
	    this.endpoint = endpoint;
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
