using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Actyx {
  public class StreamingResponse<T> : IAsyncEnumerator<T> {
    private readonly StreamReader reader;

    public StreamingResponse(Stream responseDataStream) {
      reader = new StreamReader(responseDataStream);
    }

    public T Current { get; private set; }

    public async ValueTask<bool> MoveNextAsync() {
      if (reader.EndOfStream) {
        return false;
      }

      string nextLine = await reader.ReadLineAsync();

      // Empty lines are sent as a means of keep-alive.
      while (nextLine != "event:event") {
        if (reader.EndOfStream) {
          return false;
        }

        Console.WriteLine("skipping: " + nextLine);
        nextLine = await reader.ReadLineAsync();
      }

      // Immediately after the event:event line we expect the data:{json} line
      nextLine = await reader.ReadLineAsync();
      while (!nextLine.StartsWith("data:")) {
        if (reader.EndOfStream) {
          return false;
        }

        Console.WriteLine("EXPECTED DATA BUT FOUND: " + nextLine);
        nextLine = await reader.ReadLineAsync();
      }

      // Drop the "data:" prefix and deserialize
      string jsonData = nextLine.Substring(5);
      Current = JsonConvert.DeserializeObject<T>(jsonData);

      return true;
    }

    public async ValueTask DisposeAsync() {
      await Task.Run(() => reader.Dispose());
    }
  }

  internal static class AsyncBufferExtension {
    public static async Task<IList<T>> Buffer<T>(this IAsyncEnumerable<T> stream) {
      // TODO: CancellationToken?
      var e = stream.GetAsyncEnumerator();
      IList<T> result = new List<T>();

      try {
        while (await e.MoveNextAsync()) {
          result.Add(e.Current);
        }
      } finally {
        if (e != null) {
          await e.DisposeAsync();
        }
      }

      return result;
    }
  }

  public class ActyxRequest<T> : IAsyncEnumerable<T> {
    private readonly WebRequest request;

    public ActyxRequest(WebRequest request) {
      this.request = request;
    }

    public IAsyncEnumerator<T> GetAsyncEnumerator(CancellationToken token) {
      return new StreamingResponse<T>(request.GetResponse().GetResponseStream());
    }
  }

  public class EventService {
    private readonly string authToken;
    private readonly string endpoint;

    public static async Task<EventService> ForApp(
      string appName,
      string endpoint = "http://localhost",
      int eventServicePort = 4454,
      int nodePort = 4457
      ) {
      var request = WebRequest.Create(endpoint + ':' + nodePort + "/api/v0/apps/" + Uri.EscapeUriString(appName) + "/token");

      var response = await request.GetResponseAsync();

      var reader = new StreamReader(response.GetResponseStream());

      string token = "Bearer " + JObject.Parse(reader.ReadLine())["Ok"].ToObject<string>();

      Console.WriteLine("found token: " + token);

      return new EventService(token, endpoint, eventServicePort);
    }

    public EventService(
      string authToken,
      string endpoint = "http://localhost",
      int eventServicePort = 4454
      ) {
      this.authToken = authToken;
      this.endpoint = endpoint + ':' + eventServicePort;
    }

    private WebRequest EventServiceRequest(string path) {
      Console.WriteLine(this.endpoint + path);
      WebRequest request = WebRequest.Create(this.endpoint + path);
      request.ContentType = "application/json";
      request.Headers.Add("Authorization", this.authToken);

      return request;
    }

    private WebRequest Post(string path, string postData) {
      WebRequest request = this.EventServiceRequest(path);
      // Setup POST data:
      request.Method = "POST";
      byte[] reqMsgBytes = Encoding.UTF8.GetBytes(postData);

      Stream dataStream = request.GetRequestStream();
      dataStream.Write(reqMsgBytes, 0, reqMsgBytes.Length);
      dataStream.Close();

      return request;
    }

    public async Task<Dictionary<string, ulong>> Offsets() {
      var request = this.EventServiceRequest("/api/v2/events/offsets");

      var response = await request.GetResponseAsync();

      var reader = new StreamReader(response.GetResponseStream());

      return JsonConvert.DeserializeObject<Dictionary<string, ulong>>(reader.ReadLine());
    }

    public IAsyncEnumerable<ISuttMessage> SubscribeUntilTimeTravel(
      string session,
      string subscription,
      IDictionary<string, ulong> offsets
    ) {
      var req = new {
        session,
        subscription,
        offsets
      };

      string postData = JsonConvert.SerializeObject(req);

      return new ActyxRequest<ISuttMessage>(this.Post("/api/v2/events/subscribeUntilTimeTravel", postData));
    }

    public IAsyncEnumerable<ISuttMessage> SubscribeUntilTimeTravel(string session, string subscription, params SnapshotCompression[] acceptedFormats) {
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

      return new ActyxRequest<ISuttMessage>(this.Post("/api/v2/events/subscribeUntilTimeTravel", postData));
    }

    public async Task<IList<PublishSucceeded>> Publish(IEnumerable<IEventDraft> events) {
      var r = new {
        data = events
      };
      string postData = JsonConvert.SerializeObject(r);

      var req = new ActyxRequest<PublishSucceeded>(this.Post("/api/v2/events/publish", postData));

      return await req.Buffer();
    }

    public async Task<IList<Event>> Query(string subscription,
      IDictionary<string, ulong> upperBound,
      EventsOrder order) {
      return await this.Query(subscription, new Dictionary<string, ulong>(), upperBound, order);
    }

    public async Task<IList<Event>> Query(string subscription,
      IDictionary<string, ulong> lowerBound,
      IDictionary<string, ulong> upperBound,
      EventsOrder order) {
      return await this.QueryStreaming(subscription, lowerBound, upperBound, order).Buffer();
    }

    public IAsyncEnumerable<Event> QueryStreaming(string subscription, IDictionary<string, ulong> upperBound, EventsOrder order) {
      return this.QueryStreaming(subscription, new Dictionary<string, ulong>(), upperBound, order);
    }

    public IAsyncEnumerable<Event> QueryStreaming(
      string subscription,
      IDictionary<string, ulong> lowerBound,
      IDictionary<string, ulong> upperBound,
      EventsOrder order
    ) {
      var req = new {
        subscription,
        lowerBound,
        upperBound,
        order = order.ToWireString()
      };

      string postData = JsonConvert.SerializeObject(req);
      Console.WriteLine(postData);

      return new ActyxRequest<Event>(this.Post("/api/v2/events/query", postData));
    }

    public IAsyncEnumerable<Event> Subscribe(string subscription) {
      return this.Subscribe(subscription, new Dictionary<string, ulong>());
    }

    public IAsyncEnumerable<Event> Subscribe(string subscription, IDictionary<string, ulong> lowerBound) {
      var req = new {
        subscription,
        lowerBound
      };

      string postData = JsonConvert.SerializeObject(req);

      return new ActyxRequest<Event>(Post("/api/v2/events/subscribe", postData));
    }
  }
}
