---
title: Event Subscriptions
---

_A fish can listen to more events than it generates itself, it can accumulate knowledge from many sources._

The state kept within a fish depends on the initial state, the `onEvent` handler, and of course on the precise events that are fed into that handler.
Therefore, defining the set of event subscriptions is fundamental, it belongs with the declaration of a fish; in particular, the subscription set cannot be changed dynamically, as it is so fundamental to the function of the fish.

```typescript
const chatRoomTag = Tag<ChatRoomEvent>('chatRoom')

export const mkChatRoomFish = (name: string): Fish<string[], ChatRoomEvent> => ({
  fishId: FishId.of('ax.example.ChatRoom', name, 0),
  initialState: [],
  onEvent: chatRoomOnEvent,
  where: chatRoomTag.withId(name).local(),
})
```

A fish with this definition asks for exactly those events that are tagged with `'chatRoom:my-room'`, if the fish's name is `my-room`, and only those events that have been emitted by the very same node the fish runs on (as expressed by the postfix `local()`).
This example is of course contrived because a chat room is most useful when the participants on different edge devices can see each others’ messages.
We could further refine the query with other tags, for example if we'd like to subscribe to all messages in the `broadcast` chat room as well, this time from all nodes.
The resulting query could look like the following:

```typescript
  where: chatRoomTag.withId(name).local().or(chatRoomTag.withId('broadcast'))
```

:::note
For simple cases, you can omit the type parameter for `Tag` if inlined in the fish's definition. It's best practice
however to associate event types and tags statically however using [typed tags].

[typed tags]: typed-tags
:::

The `Fish` itself is not parameterized with a `name`, that's why we created a factory function that instantiates the concrete fish
for us.  This template can be instantiated on different nodes and with different fish names, e.g. for keeping different
chat rooms separate.  The resulting fish will use the same event handler everywhere, but the
set of events it gets may be different for each instance (due to the use of the `.local()` constraint).

In order to make our chat room fish distributed and enable multiple nodes to participate in the chat, we need to change
the subscription to say that we want events from the chat room fish of the given chat room name, but from all nodes.
This is achieved by just omitting the `local()` prefix everywhere:

```typescript
  where: chatRoomTag.withId(name).or(chatRoomTag.withId('broadcast'))
```

If we'd like to get all chat room events, for all rooms, then we'd need to add another dedicated tag for that, maybe
just plain `chat`, or we re-use the `chatRoom` tag, but omitting the actual room's name.  Such a fish might make sense
for keeping track of some property across all rooms, e.g. seeing how often people use swear words in the chat.

With this modification we are ready to see what happens when we run two instances of the chat room fish on different
nodes — our first distributed app.  Doing so exhibits one of the core features of Actyx Pond: [time travel](time-travel.md).
But before we go there, we take a closer look at the relationship between tags and types.
