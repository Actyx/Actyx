---
title: Time Travel
---

Actyx Pond allows always available distributed apps to be written with any logic you like, and the result will be eventually consistent.
This magic feat is attained by employing nothing less than time travel.

To make our distributed app experiment more interesting, we change the chat room fish to actually publish the list of messages instead of their count.
With this change, the fish type definition looks like the following:

```typescript
const chatRoomSemantics = Semantics.of('ax.example.ChatRoom')
export const chatRoomFish = FishType.of({
  semantics: chatRoomSemantics,
  initialState: (name) => ({
    state: [],
    subscriptions: [{ semantics: chatRoomSemantics, name }]
  }),
  onCommand: chatRoomOnCommand,
  onEvent: chatRoomOnEvent,
  onStateChange: OnStateChange.publishPrivateState()
})
```

Now we create two small programs for interacting with the chat room.
The first one is for sending messages into it that we assume to come from the command line:

```typescript
export const main3 = (pond: Pond, message: string) =>
  pond
    .feed(chatRoomFish, 'my-room')({ type: 'addMessage', message })
    .toPromise()
```

The second one observes the state of the chat room fish and prints the list of messages whenever it changes:

```typescript
export const main4 = (pond: Pond) =>
  pond
    .observe(chatRoomFish, 'my-room')
    .do(msgs => {
      msgs.forEach(msg => console.log(msg))
      console.log('---')
    })
    .subscribe()
```

Running the observer on one device and the sender on two different devices we should see the list of messages updating as we send new messages to the fish.
The sequence of these messages corresponds to the sequence in which we send them from the two devices.
Now, if we detach one of the sender devices from the network for a bit and send some messages from it, let’s say `msgA1` to `msgA3`, while also sending `msgB1` to `msgB3` from the still connected device, we will see only `msgB1` to `msgB3` showing up at our observer.
This is because the other messages cannot be transferred right now.

When reconnecting the previously disconnected sending device, its messages `msgA1` to `msgA3` will show up at the observer after a short while.
But we notice that the messages show up not at the end of the log but interleaves with the others, for example like

    msgB1
    msgB2
    msgB3
    --- // this is the previous state
    msgB1
    msgA1
    msgA2
    msgB2
    msgA3
    msgB3
    ---

How can this be?
The `onEvent` handler only ever appends messages to the array, it never inserts them in the middle, yet we see that the state was changed “in the middle”.
The answer is that the `onEvent` handler may be invoked multiple times for the same event, if after the reception of that event other events are received that are sorted “earlier” than this event.

We recall that the current state of the fish is computed by applying one event after the other, through the `onEvent` handler.
This can be visualized like the grey zigzag line zipping together the events and their resulting states on the left-hand side of the following diagram.

![](../images/time-travel.png)

When a new event arrives, that belongs somewhere in the middle of the previously known log of events, then it is inserted in its rightful spot and the current state is recomputed by applying all events again, now including the inserted event.
This is shown on the right-hand side of the diagram above; in practice the state computation starts from the state right before the inserted event in most cases, as a cache of states is kept in memory.

This is the reason for the **very important principle that fish state never be changed**, the state computation always needs to create a fresh copy.
This copy can reuse pieces that are not changed, as is frequently done in so-called persistent data structures that are used in functional programming.

The remaining question is where the ordering of events comes from: how does the Pond know where a newly received events needs to be sorted into the event log?

The answer is that each event carries metadata including physical and logical timestamps and the unique identifier of the source of the event, so that an ordering can be defined that can be evaluated by all Ponds of a distributed application in the same fashion.
This allows all Ponds to sort the full event log in exactly the same order regardless of the order in which they received the events from their peers.
With this technique, it is the log itself that is eventually consistent and any business logic that deterministically computes state from such a log will be eventually consistent as well.

> Note
>
> This explains why it is imperative that `onEvent` be fully deterministic and only depends on its inputs: the state and the event. If for example a random number generator bound to each specific edge device were used in the computation, then the same fish would compute different states on difference devices, making the system no longer eventually consistent.

Equipped with this knowledge we are now ready to tackle local decision-making as explained in the next section on command validation.