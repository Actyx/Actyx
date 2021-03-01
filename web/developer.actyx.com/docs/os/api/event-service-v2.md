---
title: Event Service
---
<!-- textlint-disable -->
This is a reference page for the ActyxOS **Event API**.

The Event Service HTTP API provides local access to the Event Service, allowing you to

- [get the node ID](#get-node-id)
- [get information about known offsets](#get-information-about-known-offsets),
- [query event streams](#query-event-streams),
- [subscribe to event streams](#subscribe-to-event-streams),
- [subscribe to event streams monotonically](#subscribe-to-event-streams-monotonically); and,
- [publish events](#publish-events)

It is reachable at the following base URI: `http://localhost:4454/api/v2/events`.

:::info Pretty printed JSON
JSON used in the examples below is pretty-printed with [jq](https://stedolan.github.io/jq/). This is only to make it more readable here. In reality, the Event Service API does not return pretty-printed JSON but the usual compact JSON you know from any other service.
:::

## Prerequisites

Communication with the Event Service needs to be authenticated. Therefore an authorization token which is associated with the requesting app needs to be retrieved from the Console Service. This token then needs to be passed in the `Authorization` header with every request to the Event Service. In the following examples we will use the `$AUTH_TOKEN` environment variable which can be initialized with

```bash
export AUTH_TOKEN="$(curl -s localhost:4457/api/v0/apps/example_app/token | jq -r '.Ok')"
```

While the following examples use [cURL](https://curl.se/) other command-line or graphical tools (e.g. [Postman](https://www.postman.com/product/api-client/)) would work as well.

## Get the node ID

You can request the ID of the node backing the Event Service.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/node_id`
- HTTP method: `GET`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - (optional) `Accept`, must be `application/json`, default: `application/json`

There is no request body.

### Response

- HTTP headers:
  - `Content-Type` is `application/json`
  - `Cache-Control` is `no-store` (to get fresh data and not use cache slots)

The response body will contain a JSON object of the following structure:

```json
{
    "nodeId": "<string: node-id>",
}
```

### Example

See the following example using cURL:

```bash
curl \
    -s -X "GET" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Accept: application/json" \
    http://localhost:4454/api/v2/events/node_id | jq .
>
{
    "nodeId": "db66a77f"
}
```

## Get information about known offsets

You can get information from the Event Service about known offsets, i.e. what the event service believes to be the latest offset for each stream.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/offsets`
- HTTP method: `GET`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - (optional) `Accept`, must be `application/json`, default: `application/json`

There is no request body.

### Response

- HTTP headers:
  - `Content-Type` is `application/json`
  - `Cache-Control` is `no-store` (to get fresh data and not use cache slots)

The response body will contain a JSON object of the following structure:

```json
{
    "<string: sourceID>": "<integer: last-known-offset>",
    "<string: sourceID>": "<integer: last-known-offset>"
}
```

### Example

See the following example using cURL:

```bash
curl \
    -s -X "GET" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -H "Accept: application/json" \
    http://localhost:4454/api/v2/events/offsets | jq .
>
{
    "db66a77f": 57,
    "a263bad7": 60
}
```

## Query event streams

You can query the Event Service for bounded sets of events in one or more event streams.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/query`
- HTTP method: `POST`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - `Content-Type`, must be `application/json`
  - (optional) `Accept`, must be `text/event-stream`, default: `text/event-stream`

The request body must contain a JSON object with the following structure:

```json
{
    "lowerBound": {
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. 34>",
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. -1>"
    },
    "upperBound": {
        "<string: sourceID>": "<integer: inclusive-upper-bound, e.g. 49>",
        "<string: sourceID>": "<integer: inclusive-upper-bound, e.g. 101>"
    },
    "subscription": "<string: tag expression, e.g. ['tag1' & 'tag2']>",
    "order": "<string: 'lamport' | 'lamport-reverse' | 'source-ordered'"
}
```

You use the request body to specify the details of your request as documented in the following.

#### Optional: Lower bound for offsets (`lowerBound`)

The `lowerBound` object specifies the lower bound offset for each source id with the numbers being **exclusive**. i.e. a `lowerBound` specification of `34` means the event service will return events with offsets `> 34`.

The `lowerBound` is optional. If none is set for one, multiple or all subscribed sources, the Event Store will assume no lower bound.

#### Required: Upper bounds for offsets (`upperBound`)

The `upperBound` object specifies the upper bound offset for each source id with the numbers being **inclusive**. i.e. an `upperBound` specification of `34` means the event service will return events with offsets `<= 34`.

The `upperBound` is **required.** For every subscribed source where no upper bound offset is set, the result will be empty.

#### Required: Subscription (`subscription`)

The `subscription` field specifies a tag expression for which events should be queried.

// TODO: Link to subscription docs.

Not specifying the source of a stream does not make sense in this context since no events will be returned for sources without a defined upper bound.

#### Required: Ordering (`order`)

The `order` object specifies in which order the events should be returned to the caller. There are three options, one of which must be specified:

1. `lamport`: ascending order according to events' [lamport timestamp](https://en.wikipedia.org/wiki/Lamport_timestamps)
2. `lamport-reverse`: descending order according to events' lamport timestamp
3. `source-ordered`: ascending order according to events' lamport timestamp per source, with no inter-source ordering guarantees

### Response

- HTTP headers:
  - `Content-Type` is `text/event-stream`
  - `Transfer-Encoding` is `chunked`

The response will be in the [Server-Sent Events format](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) with an event identifier of `event` and a `data` field of the following structure:

```js
{
    "stream": {
        "semantics": "<string: semantics>",
        "name": "<string: name>",
        "source": "<string: sourceID>"
    },
    "timestamp": "<integer: unix epoch in microseconds>",
    "lamport": "<integer>",
    "offset": "<integer>",
    "payload": "<object>"
}
```

If an error is encountered while processing the stream of events, the stream will terminate with a final error JSON object with the following structure:

```js
{
    "error": "message",
    "errorCode": 500
}
```

TODO: verify! currently getting {"code":500,"message":"warp internal server error, unhandled Rejection."} for an empty request body while the server logs

```text
code 500 rejections Rejection([MethodNotAllowed, BodyDeserializeError { cause: Error("missing field `subscription`", line: 1, column: 2) }])
```

### Example

See the following example using cURL:

```bash
echo '
{
    "lowerBound": {
        "db66a77f": 34,
        "a263bad7": -1
    },
    "upperBound": {
        "db66a77f": 57,
        "a263bad7": 60
    },
    "subscription": "'com.actyx.examples.temperature' & ('sensor:temp-sensor1' | 'sensor:temp-sensor2')",
    "order": "lamport-reverse"
}
' \
| curl \
    -s -X "POST" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d @- \
    -H "Content-Type: application/json" \
    -H "Accept: text/event-stream" \
    http://localhost:4454/api/v2/events/query \
| grep -A1 "event:event" | grep "data:" | sed -e "s/^data://" | jq .
>
{
    "stream": {
        "semantics": "_t_",
        "name": "_t_",
        "source": "db66a77f"
    },
    "timestamp": 1599224884528020,
    "lamport": 323,
    "offset": 34,
    "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
    }
}
```

## Subscribe to event streams

You can use the Event Service API to subscribe to event streams. The Event Service may return past events and continue returning new "live" events as they are received.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/subscribe`
- HTTP method: `POST`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - `Content-Type`, must be `application/json`
  - (optional) `Accept`, must be `text/event-stream`, default: `text/event-stream`

The request body must contain a JSON object with the following structure:

```js
{
    "lowerBound": {
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. 34>",
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. -1>"
    },

    "subscription": "<string: tag expression, e.g. ['tag1' & 'tag2']>"
}
```

You use the request body to specify the details of your request as documented in the following.

#### Optional: Lower bound for offsets (`lowerBound`)

The `lowerBound` object specifies the lower bound offset for each source id with the numbers being **exclusive**. i.e. a `lowerBound` specification of `34` means the event service will return events with offsets `> 34`.

The `lowerBound` is optional. If none is set for one, multiple or all subscribed sources, the Event Store will assume a lower bound offset of `-1`, i.e. the beginning.

#### Required: Subscription (`subscription`)

The `subscription` field specifies a tag expression for which events should be queried.

### Response

- HTTP headers:
  - `Content-Type` is `text/event-stream`
  - `Transfer-Encoding` is `chunked`

The response will be in the [Server-Sent Events format](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) with an event identifier of `event` and a `data` field of the following structure:

```js
{
    "stream": {
        "semantics": "<string: semantics>",
        "name": "<string: name>",
        "source": "<string: sourceID>"
    },
    "timestamp": "<integer: unix epoch in microseconds>",
    "lamport": "<integer>",
    "offset": "<integer>",
    "payload": "<object>"
}
```

If an error is encountered while processing the stream of events, the stream will terminate with a final error JSON object with the following structure:

```js
{
    "error": "message",
    "errorCode": 500
}
```

// TODO: verify!

### Example

See the following example using cURL:

```bash
echo '
{
    "lowerBound": {
        "db66a77f": 34,
        "a263bad7": -1
    },
    "subscription": "'com.actyx.examples.temperature' & ('sensor:temp-sensor1' | 'sensor:temp-sensor2')"
}
' \
| curl -N \
    -s -X "POST" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d @- \
    -H "Content-Type: application/json" \
    -H "Accept: text/event-stream" \
    http://localhost:4454/api/v2/events/subscribe \
| grep --line-buffered -A1 "event:event" | grep --line-buffered "data:" | sed -ue "s/^data://" | jq --unbuffered .
>
{
    "stream": {
        "semantics": "_t_",
        "name": "_t_",
        "source": "db66a77f"
    },
    "timestamp": 1599224884528020,
    "lamport": 323,
    "offset": 34,
    "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
    }
}
```

## Subscribe to event streams monotonically

You can use the Event Service API to subscribe to event streams with strong ordering guarentees. This means that whenever the service learns about events that need to be sorted earlier than an event that has already been delivered the result is finished with a time travel event.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/subscribe_monotonic`
- HTTP method: `POST`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - `Content-Type`, must be `application/json`
  - (optional) `Accept`, must be `text/event-stream`, default: `text/event-stream`

The request body must contain a JSON object with one of the following structures:

#### Starting from offsets

```js
{
    "session": "<string: user supplied session ID>",
    "subscription": "<string: tag expression, e.g. ['tag1' & 'tag2']>",
    "offsets": {
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. 34>",
        "<string: sourceID>": "<integer: exclusive-lower-bound, e.g. -1>"
    },
}
```

The `offsets` object specifies the lower bound offset for each source id with the numbers being **exclusive**. i.e. a `offsets` specification of `34` means the event service will return events with offsets `> 34`.

#### Starting from a snapshot

```js
{
    "session": "<string: user supplied session ID>",
    "subscription": "<string: tag expression, e.g. ['tag1' & 'tag2']>",
    "snapshot": {
        "<string: compression>": "<string: 'none' | 'deflate'>"
    },
}
```

The `snapshot` object specifies that the event should start with returning a snapshot if there exists one. In that case, events will be returned starting from the snapshot. Otherwise, events will be returned from the beginning of time on.

The specified compression scheme will be used for delivering snapshots.

// TODO: Link to snapshot docs.

Specify additional details of your request as documented in the following.

#### Required: Session ID (`session`)

The session identifier is chosen by the client and must be used consistently by the client to resume an earlier session.

TODO: what if the user makes to requests with same session id but different subscription?

#### Required: Subscription (`subscription`)

The `subscription` field specifies a tag expression for which events should be queried.

### Response

- HTTP headers:
  - `Content-Type` is `text/event-stream`
  - `Transfer-Encoding` is `chunked`

The response will be in the [Server-Sent Events format](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) with the following event identifiers of `event` and the respective `data` formats:

#### Event type `snapshot`

This message may be sent in the beginning when a suitable snapshot has been found for this session. It may also be sent at later times when suitable snapshots become available by other means (if for example this session is computed also on a different node).

```js
{
    "type: "<string: 'snapshot'>", // TODO: isn't this reduntant with the SSE event tag?
    "compression": "<string: 'none' | 'deflate'>",
    "data": "<string: Base64 encoded snapshot>", // TODO is this correct?
}
```

#### Event type `event`

```js
{
    "type": "<string: 'event'>",
    "caughtUp": "<boolean: known events delivery exhausted?>",
    "event":  {
        "key": {
            "stream": "<string: sourceID>", // TODO: this is inconsistent with /subscribe
            "lamport": "<integer>",
            "offset": "<integer>"
        },
        "meta" {
          "timestamp": "<integer: unix epoch in microseconds>",
          "tags": ["<string: tag, e.g. tag1>", "<string: tag, e.g. tag2>"]
        },
        "payload": "<object>"
    }
}
```

#### Event type `timeTravel` // TODO still use this term?

In case the service learns about events that need to be sorted earlier than an event that has already been delivered, an event of this type is emitted and the stream is closed.

```js
{
    "type": "<string: 'timeTravel'>",
    "newStart": {
        "stream": {
            "semantics": "<string: semantics>",
            "name": "<string: name>",
            "source": "<string: sourceID>"
        }
        "lamport": "<integer>",
        "offset": "<integer>"
    }
}
```

If an error is encountered while processing the stream of events, the stream will terminate with a final error JSON object with the following structure:

```js
{
    "error": "message",
    "errorCode": 500
}
```

// TODO: verify!

### Example

See the following example using cURL:

```bash
echo '
{
    "session": "<my_session_id>",
    "subscription": "'com.actyx.examples.temperature' & ('sensor:temp-sensor1' | 'sensor:temp-sensor2')",
    "offsets": {
        "db66a77f": 34,
        "a263bad7": -1
    }
}
' \
| curl -N \
    -s -X "POST" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d @- \
    -H "Content-Type: application/json" \
    -H "Accept: text/event-stream" \
    http://localhost:4454/api/v2/events/subscribe_monotonic \
| grep --line-buffered -A1 "event:(snapshot\|event\|timetravel)" | grep --line-buffered "data:" | sed -ue "s/^data://" | jq --unbuffered .
>
{
    "type": "event",
    "key": {
        "lamport": 323,
        "stream": "db66a77f",
        "offset": 34,

    },
    "payload": {
        "foo": "bar",
        "fooArr": ["bar1", "bar2"]
    },
    "caughtUp": true
}
```

## Publish events

You can publish new events using the Event Service API.

### Request

- Endpoint: `http://localhost:4454/api/v2/events/publish`
- HTTP method: `POST`
- HTTP headers:
  - `Authorization`, see [Prerequisites](#prerequisites)
  - `Content-Type`, must be `application/json`

The request body must contain a JSON object with the following structure:

```js
{
    "data": [
        {
            "tags": ["<string: tag, e.g. tag1>", "<string: tag, e.g. tag2>"],
            "payload": "<object>"
        },
        {
            "tags": ["<string: tag, e.g. tag1>", "<string: tag, e.g. tag2>"],
            "payload": "<object>"
        }
    ]
}
```

// TODO: talk about tags?

### Response

The response will provide feedback using HTTP status codes, with `201` signifying that the request was successfully processed and the events published.

### Example

See the following example using cURL:

```bash
echo '
{
    "data": [
        {
            "tags": ["com.actyx.examples.temperature", "sensor:temp-sensor1"],
            "payload": {
                "foo": [1, 3, 4],
                "bar": { "a": 1, "b": 103 }
        },
        {
            "tags": ["com.actyx.examples.temperature", "sensor:temp-sensor1"],
            "payload": {
                "foo": [3, 1, 1],
                "bar": { "a": 13, "b": 48 }
        }
    ]
}
' \
| curl \
    -s -X "POST" \
    -H "Authorization: Bearer $AUTH_TOKEN" \
    -d @- \
    -H "Content-Type: application/json" \
    http://localhost:4454/api/v2/events/publish
> Response: HTTP 201 | 500 | 400 with an invalid body
```

## Usage examples in different languages

// TODO: shall these be adapted or shall we just links to the SDKs?
