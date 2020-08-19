---
title: Typed Tags
---

In the examples so far, we have not paid much attention to the type parameter `E` of `Tag<E>`. When constructing simple
tag queries inline into the fish's definition, we can omit the explicit type parameter, as it's inferred for us by the
compiler to be the same as the events the fish is able to consume in `onEvent`. If more elaborate queries are needed, we
can add an explicit type cast:

```typescript
// Only events for `my-room`
where: Tag('chatRoom').withId('my-room') // equivalent to `Tag('chatRoom:my-room')`
// Events from either `my-room` or `broadcast`
where: Tag('chatRoom:my-room').or(Tag('chatRoom:broadcast')) as TagUnion<ChatEvent> // explicit cast necessary
// Events for a specific room from a specific sender
where: Tags('chatRoom:Melmac', 'sender:Alf')
```

:::note
The `Tags` function is a shortcut to construct an intersection between multiple tags, e.g. `Tags('a', 'b')` requires
both `a` and `b` to be present on the events.
:::

These events could be emitted as follows:

```typescript
pond.emit(
  ['chatRoom:Melmac', 'sender:Alf'],
  { type: 'messageAdded', message: "If you love something, let it go. If it comes back to you, it's yours. If it's run over by a car, you don't want it." }
).toPromise()
```

And our `chatRoomFish` which subscribes to those events:

```typescript
export const mkChatRoomFish: (name: string): Fish<string[], ChatRoomEvent> => ({
  // ...
  fishId: FishId.of('ax.example.ChatRoom', name, 0),
  where: Tag('chatRoom').withId(name}),
})
```

Now, we could theoretically change the shape of `ChatRoomEvent`, requiring for example another field. If we forgot to
change the code path where events are being emitted, the events subscribed to by our `ChatRoomFish` would not have the
expected shape. In order to be able to link tags and event types at compile time, we have introduced the Typed Tags API.
It's a fluent API to both tag events and describe event subscriptions.

Let's see how we can rewrite our example above:

```typescript
const tags = {
  chatRoom: Tag<ChatRoomEvent>('chatRoom'),
  sender: Tag('sender'),
}

const mkChatRoomFish = (name: string): Fish<string[], ChatRoomEvent> => ({
  fishId: FishId.of('ax.example.ChatRoom', name, 0),
  initialState: [],
  onEvent: (s: string[]) => s,
  where: tags.chatRoom.withId(name),
})

export const ChatRoom = {
  mkChatRoomFish,
  tags,
}
```

First, we create a helper object called `tags`, being a single place where we can put all related tags to this fish. We wrap
both `tags` and the `mkChatRoomFish` function inside a wrapper object called `ChatRoom` and export only that. This way,
all coherent parts are modularized. The subscription of the fish can be rewritten to `tags.chatRoom.withId(name)`,
which will require all events to have the tag `` `chatRoom:${name}` ``. We're ignoring the `sender` tag within the
subscription, as every event we're interested should at least have the `chatRoom` tag. Also note, that we parameterized
the chat room tag with the type `ChatRoomEvent`.

Now, how will this help us?

```typescript
pond.emit(
  ChatRoom.tags.chatRoom.withId('Melmac').and(ChatRoom.tags.sender.withId('Alf')),
  { type: 'this type does not exist' }
).toPromise()
```

This will now actually fail to compile, because only a `ChatRoomEvent` is allowed to be passed to the `emit` function. The
same mechanism can be used as well for state effects.

Finally, let's go back to our initial queries and rewrite them using the fluent API:

```typescript
const tags = {
  chatRoom: Tag<ChatRoomEvent>('chatRoom'),
  sender: Tag<ChatRoomEvent>('sender'),
}
// TagQuery.requireAll('chatRoom:my-room')
tags.chatRoom.withId('my-room')
// TagQuery.matchAnyOf('chatRoom:broadcast', 'chatRoom:my-room')
tags.chatRoom.withId('broadcast').or(tags.chatRoom.withId('my-room'))
// TagQuery.requireAll('chatRoom:Melmac', 'sender:Alf')
tags.chatRoom.withId('Melmac').and(tags.sender.withId('Alf'))
```
