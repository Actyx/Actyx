---
title: Event Streams
---

Publishing and receiving events is the most fundamental way in which you program collaboration with ActyxOS.

ActyxOS is all about collaboration between apps installed on edge devices. The primary method of doing so is based on so-called _persistent event streams_. Let's walk through the basics of persistent event streams and how they work in ActyxOS.

## Basics

Events are facts from the past that can represent anything you would like them to: a robot arriving at a location, a person signing off from work or a button click. Event streams are simply ordered chains of events. Event streams are very valuable because they allow you to keep track of what is happening and make decisions accordingly&#8211;especially in other parts of the system.

Consider a robot moving from place to place. This robot may publish an event stream about itself so that other machines or humans can make decisions accordingly:

```javascript
[
	{
	    source: 'robot1',
	    timestamp: 1568807572258000,
	    payload: { locationChangedTo: 'loading-bay-1' }
	},
	{
	    source: 'robot1',
	    timestamp: 1568807936074000,
	    payload: { locationChangedTo: 'charging-station-39' }
	}
]
```

Technically, event streaming is a message-driven, publish and subscribe mechanism that enables asynchronous communication between several components, as upstream components transfer processing to downstream components by publishing domain events that are then consumed downstream. Event streaming enables event dissemination throughout the entire system.

With most event streaming technologies events are ephemeral, meaning that the events are only available for a short period of time. After this period, interested parties will no longer be able to find or read this event. _Persistent event streams_ expand on this by automatically persisting published events (to disk, for example). This means interested consumers can access any past event at any time.

## Creating persistent event streams with ActyxOS

The ActyxOS [Event Service](/os/docs/event-service.html) gives you the ability to create persistent event streams. This means you can publish events, subscribe to event streams and query event streams, including asking for events from long ago.

ActyxOS automatically persists published events and disseminates them to all other edge devices (and apps running on them) in a peer-to-peer fashion. As a producer or consumer of events you only need to interact with the local Event Service (using the [Event Service API](/os/docs/event-api.html)).

To create clarity about event sources, meaning and stream names, ActyxOS identifies a stream by a three-tuple, namely:

1. **source** - the device generating the event stream
2. **semantics** - the meaning of the stream
3. **name** - the name of the stream

Let's consider, for instance, the robot example above. This robot sends events about its location changes. The app running on the robot would have a piece of code as follows for generating relevant event objects (more information about this in the [Event Service Guide](/os/docs/event-service.html)).

```typescript
function mkPositionChangedEvent(newPosition: string): Event {
  return {
      semantics: "com.robot-maker-ltd.positionChange",
      name: "robot1",
      payload: { locationChangedTo: newPosition }
  }
}
```

Whenever the robot has changed its position it would then publish the relevant events.

```typescript{11-12}
function publishEvent(event: Event): void {
  return fetch("http://localhost:4454/api/v1/events/publish", {
    method: "POST",
    body: JSON.stringify({ data: [event] }),
    headers: { "Content-Type": "application/json" }
  });
}

// This function would be provided to call to a higher-level controller
function onChangedPosition(newPosition: string): void {
  const event = mkChangedPositionEvent(newPosition);
  publishEvent(event);
}
```

> Note
>
> We did not have to specify the source for this event publication. That is because the local Event Service (at `http://localhost:4454`), to which we are publishing the event,  will automatically add the device's source ID.

Because ActyxOS automatically disseminates events, on a second device, at any time, you could subscribe to that specific event stream, and receive all events published by the robot.

```javascript{4,17-29}
fetch("http://localhost:4454/api/v1/events/subscribe", {
  method: "POST",
  body: JSON.stringify({
    subscriptions: [{ name: "robot1" }]
  }),
  headers: { "Content-Type": "application/json" }
})
  .then(r => r.body.getReader())
  .then(reader => {
    const dec = new TextDecoder();
    const loop = () => {
      reader.read().then(
        chunk => {
          if (!chunk.done) {
            console.log("Received event from robot:", JSON.parse(dec.decode(chunk.value)));
            // Result:
            // {
            //     "stream": {
            //         "semantics": "com.robot-maker-ltd.positionChange",
            //         "name": "robot1",
            //         "source": "db66a77f"
            //     },
            //     "timestamp": 21323,
            //     "lamport": 323,
            //     "offset": 34,
            //     "payload": {
            //         "locationChangedTo": "loading-bay-5"
            //     }
            // }
            //
            loop();
          }
        }
      );
    };
    loop();
  });
```

> Note
>
> Did you notice how in the last example we specified the event stream's name in our subscription? This is important because we are accessing the stream from another device and need to tell the Event Service what we are interested in. The ActyxOS Event Service provides powerful subscription mechanisms based on the stream source, semantics and name. Check out the [Event Service](/os/docs/event-service.html) guide for more information.

As noted above, the ActyxOS Event Service also persists the events upon publication. This means that we can not only access current and future events, but also events from the past. This is an important property for event sourcing; one of the most popular methods for building apps on Actyx. Check out [Event Sourcing](/os/docs/event-sourcing.html) for more information.

## Learn more

- Read the [advanced guide](/os/docs/event-service.html) for the Event Service
- Refer to the Event Service [API Reference](/os/docs/event-api.html)

Or, go to the next section to learn more about how to store and distribute data blobs with ActyxOS.
