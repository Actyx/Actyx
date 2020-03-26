---
title: Command validation
---

Fishes can record not only facts from sensors, they can also create facts by recording decisions.

> If you only read one thing, read the note further down on eventual consistency!

In fact, what we have done so far in the chat room example was just that: the chat room fish has recorded the decisions we made before sending the respective commands.
This is a common theme in that a fish’s observable state may be used to drive a UI from which a human operator selects possible actions.
Each action is then recorded as a fact by emitting a corresponding event.

The UI will typically only display valid actions for the current state, so some validation is already performed within this loop.
The Fish API allows us to add more validation by checking the incoming command against the current state before deciding whether and which events to emit.

In the chat room example we may forbid the posting of a message that has already been posted (this is a slightly contrived example, but sometimes Slack might be better with such a policy).
We can implement this by making use of the current state that is passed into the `onCommand` handler.

```typescript
const chatRoomOnCommand: OnCommand<string[], ChatRoomCommand, ChatRoomEvent> = (state, command) => {
  switch (command.type) {
    case 'addMessage': {
      const { message } = command
      if (state.includes(message)) {
        return []
      }
      return [{ type: 'messageAdded', message }]
    }
    default:
      return unreachableOrElse(command.type, [])
  }
}
```

When trying this out, it will work to our satisfaction as long as messages are posted one after the other from different devices while the network is fully working.
If we disconnect one device from the network and send message `Hello World!`, then send that very same message from a still connected device, we will see the second message accepted; worse still, when reconnecting the previously disconnected device, we will see both copies of the message in the log.

The reason for this is as simple as it is fundamental: each fish can make decisions only based on the incomplete knowledge that it has.
During a network partition, or when things happen truly concurrently, the knowledge may be crucially incomplete, leading to wrong inputs being accepted.
Once an event is in the log there is no way to remove it again, so we will have to live with the mistake — **command validation does not run again during time travel.**

The good thing is that we can easily recognize the mistake in the `onEvent` handler or by inspecting the observed state.
The second message could be displayed differently or not at all, as shown in the following snippet.

```typescript
const chatRoomOnEvent: OnEvent<string[], ChatRoomEvent> = (state, event) => {
  const { payload } = event
  switch (payload.type) {
    case 'messageAdded': {
      const { message } = payload
      if (state.includes(message)) {
        return [...state, `[${message}] <-- DUPLICATE`]
      }
      return [...state, message]
    }
    default:
      return unreachableOrElse(payload.type, state)
  }
}
```

Instead of showing the data differently, we might just as easily observe the duplication in the state and send a text message to alert the operator of the chat room — this would not make much sense in this example use-case, but in different cases where for example a logistics robot discovers that it is delivering material to a factory workstation that already received that delivery from another robot, it would make sense to alert a human to resolve the conflict.

> Important note
>
> The fact that command validation in Actyx Pond is not strictly consistent is the price that needs to be paid for having a system that is 100% available, where all devices can always make progress independent from each other.
> Distributed systems research shows that it is impossible to make this consistent without having to stop the system during certain network partitions or hardware outages.

With this, we have discussed all concepts that are needed to master the distributed side of apps on ActyxOS with Actyx Pond.
When creating multiple communicating fishes or evolving an existing app, the event schema needs to be kept compatible, thus we take a closer look at the type parameters of FishType in the next section.
