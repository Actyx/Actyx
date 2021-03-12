Websocket RPC protocol
======================

The protocol here allows clients to use a single websocket connection to call multiple streaming APIs from a backend.
It is possible to call different API endpoints or different invocations of the same API and receive a multiplexed
stream containing the response streams for each of these calls.

Initiating a call
-----------------

The client initiates a call by sending the following message to the server over the websocket connection:

```json
{"type":"request","serviceId":"getCustomerIds","requestId":652,"payload":{ ... }}
```

The `type` field indicates that this is an API request, the `serviceId` field indicates that we are about to invoke the
`getCustomerIds` service. The `requestId` field is used for multiplexing: each answer belonging to this particular call
will bear the `requestId` of 652. Finally, the `payload` field contains the JSON formatted message that is to be
parsed by the particular service, in this case `getCustomerIds`.

If the `requestId` was already used by a previous request, then that request is implicitly cancelled.

On a successful call, the backend will respond with a stream of messages:

```json
{"type":"next","requestId":652,"payload":{...}}
{"type":"next","requestId":652,"payload":{...}}
{"type":"next","requestId":652,"payload":{...}}
{"type":"next","requestId":652,"payload":{...}}
```

Each of these messages might be interleaved with arbitrary many other streams, however they will not share the same 
`requestId`. It is the responsibility of the client to demultiplex these streams properly by using the `requestId` field.

The `payload` field contains the JSON formatted response of the particular service, the format of which is defined by
the service itself but is completely opaque to the multiplexing protocol.

Once the backend completes the stream (this is not necessary, some streams might be infinite) it sends the following message:

```json
{"type":"complete","requestId":652}
```

Cancellation
------------

The client can always cancel an ongoing streaming response by sending a message of the following format:

```json
{"type":"cancel","requestId":652}
```

**Warning!** Since there is an inherent race involved here, there might be still responses inside buffers that has been
enqueued before the cancellation arrived to the server. This means that the client should be prepared to throw away such 
stray messages after it has sent the cancel message.

Errors
------

In the case the client sends a request to a non-existing service, for example:

```json
{"type":"request","serviceId":"getCustomerIdsWrong","requestId":652,"payload":{ ... }}
```

The server responds with an error of `unknownEndpoint`:

```json
{"type":"error","requestId":652,"kind":{"type":"unknownEndpoint","endpoint":"getCustomerIdsWrong"}}
```

In the case if the client sends a request that contains a payload format that cannot be deserialized to the format
the particular service expects:

```json
{"type":"request","serviceId":"getCustomerIds","requestId":49,"payload":{"bad_field_name":4}}
```

The server responds with an error of `badRequest`:

```json
{"type":"error","requestId":49,"kind":{"type":"badRequest"}}
```

In the case, where the service exists, the request is in the right format, but the request itself does not pass
validation:

```json
{"type":"request","serviceId":"getCustomerIds","requestId":49,"payload":{"customer":"Johnny"}}
```

The server responds with an error type (`serviceError`) that wraps the actual serialized error message from the service:

```json
{"type":"error","requestId":49,"kind":{"type":"serviceError","value":{"unknown_customer":"Johnny"}}}
```

In the case where the server encounters an unexpected error (i.e. a bug), it replies with `internalError`:

```json
{"type":"error","requestId":49,"kind":{"type":"internalError"}}
```