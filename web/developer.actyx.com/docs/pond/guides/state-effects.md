---
title: State Effects
hide_table_of_contents: true
---

_Events record facts, which can be a sensor reading as well as a decision that has been taken_.

:::note
If you only read one thing, read the note further down on eventual consistency!
:::

With sensor readings, like from a thermometer, it is quite obvious how they are turned into events: the app reads the sensor and emits an event with the details.
The only freedom in this process is when to trigger this action, all other details are fixed.

In our chat example, we encountered a somewhat different setting. Here it is the caller of the message emission program that determines the contents of the event, namely the message text.
We kept things simple by using the command line approach, but usually such a program has a graphical user interface where someone can enter the message and press the send button.
When that happens, the emission of the event can be seen as deterministic as it was in the thermometer case: all the details are fixed, the event records the fact that the end-user has sent the given message.

In many cases, the end-user has less freedom than in the chat app, because certain actions may be allowed only in specific situations.
In the chat case the administrator may suspend the ability to send messages, in which case the SEND button should be deactivated.
This is a common pattern that holds with a human in the loop as well as for a completely automated decision making process.

The purpose of Actyx Pond is to concentrate on the business rules and take away as much of the boilerplate as possible.
One tricky problem to solve in an event-based system is “have I done this already?” in order to avoid taking the same decision twice — it may take a bit of time to see the event that recorded the decision reach all relevant places.
In the case of a single fish, this problem is solved by using _state effects_, a mechanism that allows you to atomically check the latest state and decide upon the emission of some events.
The Pond guarantees that the next state effect will only run once the emitted events have been applied to the state.

In the chat room example we may demonstrate this by forbidding the posting of a message that has already been posted (this is a slightly contrived example, but sometimes Slack might be better with such a policy).
We can implement this by making use of the current state that is passed into the effect function.

```typescript
const sendChatMessage = (message: string): StateEffect<string[], ChatRoomEvent> =>
  (state, enqueue) => {
    if (!state.includes(message)) {
      enqueue(Tags('chatRoom:my-room'),  { type: 'messageAdded', message })
    }
  }

```

When trying this out, it will work to our satisfaction as long as messages are posted one after the other from different devices while the network is fully working.
If we disconnect one device from the network and send message `Hello World!`, then send that very same message from a still connected device, we will see the second message accepted; worse still, when reconnecting the previously disconnected device, we will see both copies of the message in the log.

The reason for this is as simple as it is fundamental: each fish can make decisions only based on the incomplete knowledge that it has.
During a network partition, or when things happen truly concurrently, the knowledge may be crucially incomplete, leading to wrong inputs being accepted.
Once an event is in the log there is no way to remove it again, so we will have to live with the mistake — **the effect function does not run again during time travel.**

The good thing is that we can easily recognize the mistake in the `onEvent` handler or by inspecting the observed state.
The second message could be displayed differently or not at all, as demonstrated in the following snippet.

```typescript
const chatRoomOnEvent: Reduce<string[], ChatRoomEvent> = (state, event) => {
  switch (event.type) {
    case 'messageAdded': {
      const { message } = event
      if (state.includes(message)) {
        return [...state, `[${message}] <-- DUPLICATE`]
      }
      return [...state, message]
    }
    default:
      return unreachableOrElse(event.type, state)
  }
}
```

Instead of showing the data differently, we might just as easily observe the duplication in the state and send a text message to alert the operator of the chat room — this would not make much sense in this example use-case, but in different cases where for example a logistics robot discovers that it is delivering material to a factory workstation that already received that delivery from another robot, it would make sense to alert a human to resolve the conflict.

:::warning Important note
The fact that command validation in Actyx Pond is not strictly consistent is the price that needs to be paid for having a system that is 100% available, where all devices can always make progress independent from each other.
Distributed systems research shows that it is impossible to make this consistent without having to stop the system during certain network partitions or hardware outages.
:::

With this, we have discussed all concepts that are needed to master the distributed side of apps on ActyxOS with Actyx Pond.
When creating multiple communicating fishes or evolving an existing app, the event schema needs to be kept compatible, thus we take a closer look at the type parameters of a `Fish` in the next section.
