---
title: Typed Tags
hide_table_of_contents: true
---

In the examples so far, we have not paid much attention to the type parameter `E` of `Tag<E>`. When constructing simple
tag queries inline into the fish's definition, we can omit the explicit type parameter, as it's inferred for us by the
compiler to be the same as the events the fish is able to consume in `onEvent`. If more elaborate queries are needed, we
can add an explicit type cast:

```typescript
// Only events for `my-room`.
// Equivalent to `Tags('chatRoom', 'chatRoom:my-room')`
where: Tag('chatRoom').withId('my-room')

// Events from either `my-room` or `broadcast`
// An explicit cast is needed in this case.
where: Tag('chatRoom').withId('my-room').or(
    Tag('chatRoom').withId('broadcast')
) as Where<ChatRoomEvent>

// Events for a specific room and from a specific sender
where: Tag('chatRoom').withId('Melmac').and(Tag('sender').withId('Alf'))
```

These events could be emitted as follows:

```typescript
pond.emit(
  Tag('chatRoom').withId('Melmac').and(Tag('sender').withId('Alf')),
  { type: 'messageAdded', message: "If you love something, let it go." }
)
```
:::note General and specific tags
It’s important to use the `withId` helper. It makes sure we are not only tagging with
`chatRoom:Melmac`, but with just `chatRoom` as well. This is important, because there is no
way to prefix-query tags! If a consumer wants to read all `chatRoom` events, not just those for a
specific room, it can then do so via just `Tag('chatRoom')`.
:::note


And our `chatRoomFish` which subscribes to those events:

```typescript
export const mkChatRoomFish: (name: string): Fish<string[], ChatRoomEvent> => ({
  // ...
  fishId: FishId.of('ax.example.ChatRoom', name, 0),
  where: Tag('chatRoom').withId(name),
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
  sender: Tag<ChatRoomEvent>('sender'),
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
which will require all events to have the tags `` 'chatRoom' & `chatRoom:${name}` ``. We're ignoring the `sender` tag within the
subscription, as every event we're interested should at least have the `chatRoom` tag. Also note, that we parameterized
the chat room tag with the type `ChatRoomEvent`.

Now, how will this help us?

```typescript
const tags = ChatRoom.tags

// Will fail to compile:
pond.emit(
  tags.chatRoom.withId('Melmac').and(tags.sender.withId('Alf')),
  { type: 'this type does not exist' }
)
```

This will now actually fail to compile, because only a `ChatRoomEvent` is allowed to be passed to the `emit` function.

Finally, let's go back to our initial queries and rewrite them using the typed API:

```typescript
const tags = {
  chatRoom: Tag<ChatRoomEvent>('chatRoom'),
  sender: Tag<ChatRoomEvent>('sender'),
}
// 'chatRoom' & 'chatRoom:my-room'
where: tags.chatRoom.withId('my-room')

// 'chatRoom' & ('chatRoom:broadcast' | 'chatRoom:my-room')
where: tags.chatRoom.withId('broadcast').or(tags.chatRoom.withId('my-room'))

// 'chatRoom' & 'chatRoom:Melmac' & 'sender' & 'sender:Alf'
where: tags.chatRoom.withId('Melmac').and(tags.sender.withId('Alf'))
```

:::tip Inspect your queries
Since Pond 2.2 you can call `toString()` on your `Where` objects to find out what your query does
under the hood, at a glance. The format is similar to the last code snippet’s comments.
:::
