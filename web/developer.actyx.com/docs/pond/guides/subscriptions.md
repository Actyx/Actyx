---
title: Event subscriptions
---

A fish can listen to more events than it generates itself, it can accumulate knowledge from many sources.

The fish in the previous section had provisions for updating its state upon reception of events, but it did not actually ask for any events to be delivered.
In the same way that the event stream emitted by a fish is identified with the name of the fish, the set of event streams a fish requests identifies the state that the fish will compute using its `onEvent` handler.
Therefore, defining this subscription set is fundamental, it belongs with the declaration of a fish; in particular, the subscription set cannot be changed dynamically, as it is so fundamental to the function of the fish.

```typescript
const chatRoomSemantics = Semantics.of('ax.example.ChatRoom')
export const chatRoomFish = FishType.of({
  semantics: chatRoomSemantics,
  initialState: (name, sourceId) => ({
    state: [],
    subscriptions: [{ semantics: chatRoomSemantics, name, sourceId }]
  }),
  // ... others as before
})
```

A fish with this definition asks for exactly the event stream that it emits itself: the own semantics and name as well as the sourceId of the local node.
The `name` and `sourceId` are passed as arguments to the `initialState` function because a FishType is not a single fish but a template that can be instantiated on different nodes and with different fish names, e.g. for keeping different chat rooms separate.
The resulting fish will use the same handlers for commands, events, and state everywhere, but the set of events it gets may be different for each instance.

In order to make our chat room fish distributed and enable multiple nodes to participate in the chat, we need to change the subscription to say that we want events from the chat room fish of the given chat room name, but from all nodes.
This is achieved by leaving off the `sourceId` property in the subscription definition:

```typescript
  initialState: (name) => ({
    state: [],
    subscriptions: [{ semantics: chatRoomSemantics, name }]
  }),
```

If we left off the name as well, then the fish would get all chat room events, for all rooms.
Such a fish might make sense for keeping track of some property across all rooms, e.g. seeing how often people use swear words in the chat.

With this modification we are ready to see what happens when we run two instances of the chat room fish on different nodes — our first distributed app.
Doing so exhibits one of the core features of Actyx Pond: time travel.
