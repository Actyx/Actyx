---
title: Event Subscriptions
hide_table_of_contents: true
---

_A fish can accumulate knowledge from many sources._

The state kept within a fish depends on the initial state, the `onEvent` handler, and of course on the precise events that are fed into that handler.
Therefore, defining the set of event subscriptions is fundamental, it belongs with the declaration of a fish; in particular, the subscription set cannot be changed dynamically, as it is so fundamental to the function of the fish.

```typescript
const mkChatRoomFish = (name: string): Fish<string[], ChatRoomEvent> => ({
  fishId: FishId.of('ChatRoom-Example', name, 0),
  initialState: [],
  onEvent: chatRoomOnEvent,
  where: Tag('chatRoom').withId(name),
})
```

We can use this factory function to create a fish for a specific room: `mkChatRoomFish('my-room')`.
This fish will ask for exactly those events that are tagged with `'chatRoom' & 'chatRoom:my-room'`.

We could further refine the query with other tags, for example if we'd like to subscribe to all
messages in the `broadcast` chat room as well. The resulting query:

```typescript
  where: chatRoomTag.withId(name).or(chatRoomTag.withId('broadcast'))
```

`mkChatRoomFish` can be called with different fish names. While its `onEvent` function is always the
same, the computed states will differ, because the selected set of events differs. Hence it is also
important to generate different `FishId`: Otherwise the pond might use a cached fish from a
different chat room!

If we'd like to get all chat room events, for all rooms, we just omit the call to `withId`:
`Tag('chatRoom')` selects across all ids. Such a subscription might make sense for keeping track of
some property across all rooms, e.g. seeing how often people use swear words in the chat.


With this modification we are ready to see what happens when we run two instances of the chat room fish on different
nodes — our first distributed app.  Doing so exhibits one of the core features of Actyx Pond: [time travel](time-travel.md).
But before we go there, we take a closer look at the relationship between tags and types.
