---
title: Local State
---

Making sense of events: building up local state.

A fish’s purpose is to learn about its environment and make or record decisions based on its knowledge.
So far we have only outfitted our fishes wih an initial state, their knowledge was static.
In this section we implement the logic for how a fish can change its state by ingesting information in the form of events.

As with commands, a fish reacts to events as they become available, therefore we need to add an `onEvent` handler to our fish.
Since the point of the event handler is to manage the fish’s state, the first piece of information we need to settle on is what state we want to keep.
In our example we model a chat room, so we’ll go with an array of strings.

```typescript
const chatRoomOnEvent: OnEvent<string[], ChatRoomEvent> = (state, event) => {
  const { payload } = event
  switch (payload.type) {
    case 'messageAdded': {
      return [...state, payload.message]
    }
    default:
      return unreachableOrElse(payload.type, state)
  }
}
```

> Note
>
> All necessary imports (like `OnEvent`) are available from the `@actyx/pond` module.

The only event our chat room fish knows is of type `messageAdded`, so we need to handle that.
The type of the `event` parameter to our function is not the bare event, though, it is an `Envelope<ChatRoomEvent>` that contains some metadata on the event in addition to the event payload (like where and when the event was recorded).

It is **very important to note** that the `onEvent` handler computes a new state from the old state and the event, meaning that the old state **must not be changed**.
We will get to the reason behind this important principle when discussing [time travel](time-travel).
This is the reason for using array spread syntax when creating the new fish state.
As previously, the `unreachableOrElse` helper function ensures that we don’t forget to handle a case in the switch statement.

What remains to be done is to hook this new `onEvent` handler into our fish definition:

```typescript
export const chatRoomFish = FishType.of({
  semantics: Semantics.of('ax.example.ChatRoom'),
  initialState: () => ({
    state: []
  }),
  onCommand: chatRoomOnCommand,
  onEvent: chatRoomOnEvent,
  onStateChange: OnStateChange.publishState(state => state.length)
})
```

In order to make this type-check correctly, we needed to change the `onCommand` handler’s declaration to accept the new state of type `string[]`.
Additionally, we add a state handler that publishes information about the accumulated state for the outside world to observe, here we demonstrate that this can be a function of the state, like the length of the array of accumulated messages in the chat room.

With this we have seen all important triggers for a fish: it reacts to commands by possibly emitting events, and it reacts to events by computing a new state, plus it can make a projection of its state observable by the outside world. In the next section we start looking into the distributed aspects of writing fishes.
