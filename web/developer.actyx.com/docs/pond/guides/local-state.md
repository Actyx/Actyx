---
title: Local State
---

Making sense of events: building up local state.

A fish’s purpose is to learn about its environment and make or record decisions based on its knowledge.
So far we have only outfitted our fishes wih an initial state, their knowledge was static.
In this section we implement the logic for how a fish can change its state by ingesting information in the form of events.

A fish reacts to events as they become available, therefore we need to add an `onEvent` handler to our fish.
Since the point of the event handler is to manage the fish’s state, the first piece of information we need to settle on is what state we want to keep.
In our example we model a chat room, so we’ll go with an array of strings.

```typescript
const chatRoomOnEvent: Reduce<string[], ChatRoomEvent> = (state, event) => {
  switch (event.type) {
    case 'messageAdded': {
      state.unshift(event.message)
      return state
    }
    default:
      return unreachableOrElse(event.type, state)
  }
}
```

:::note
All necessary imports (like `Reduce`) are available from the `@actyx/pond` module.
:::

The only event our chat room fish knows is of type `messageAdded`, so we need to handle that.

The `onEvent` handler computes a new state from the old state and the event; you are free to choose whether to mutate
the passed in state or not, depending on your preference.  As previously, the `unreachableOrElse` helper function
ensures that we don’t forget to handle a case in the switch statement.

What remains to be done is to hook this new `onEvent` handler into our fish definition:

```typescript
export const chatRoomFish: Fish<string[], ChatRoomEvent> = {
  fishId: FishId.of('ax.example.ChatRoom', 'lobby', 0),
  initialState: [],
  onEvent: chatRoomOnEvent,
  where: Tag('chatRoom').withId('lobby'),
}
```

With this we have seen all important triggers for a fish: it reacts to events by computing a new state, plus it can make its state observable to the outside world. In the next section we start looking into the distributed aspects of writing fishes.
