---
title: Events
---

A fish consumes events, but emitting events is not coupled to fishes.

Events can be emitted directly using the Actyx Event Service, or using the Actyx Pond. Actyx Pond provides two main APIs
for emitting events: `pond.emit` and `pond.run` (we'll learn how to use [state effects] later).
[state effects]: state-effects

Events are tagged with an arbitrary amount of tags, each tag being simply just a string:
```typescript
await pond.emit(Tags('chat', 'channel:lobby', 'sender:Alf'), { type: 'messageAdded', message: 'Hello!' }).toPromise()
```
This event will be tagged with three tags: `chat`, `channel:lobby`, and `sender:Alf`. We already see, that we can add
some structure to the tags, e.g. using the `channel` identifier.

Obviously, within the context of Actyx Pond, events usually go together with some business logic implemented in fishes.
In this example we consider a fish that models a chat room. The main action that can be performed on such a room is to
add a new message. The straight-forward definition given above will emit such an event according to the following event
type:
```typescript
type ChatRoomEvent = { type: 'messageAdded', message: string }
```
:::note
All necessary imports (like `Tags`) are available from the `@actyx/pond` module.
:::

We'll see in the next section about [local state], how to construct a fish to make use of the emitted event.

[local state]: local-state
