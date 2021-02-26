---
title: Time Travel
hide_table_of_contents: true
---

_Actyx Pond allows always available distributed apps to be written with any logic you like, and the result will be eventually consistent.
This magic feat is attained by employing nothing less than time travel._

Recall our chat room fish that accumulates messages in an array of strings:

```typescript
export const mkChatRoomFish = (name: string) => ({
  fishId: FishId.of('ax.example.ChatRoom', name, 0),
  initialState: [],
  onEvent: chatRoomOnEvent,
  where: Tag('chatRoom').withId(name),
})
```

Now we create two small programs for interacting with the chat room.
The first one is for sending messages into it (perhaps coming from the command line):

```typescript
const sendMessage = (pond: Pond, message: string) =>
  pond.emit(
    Tag('chatRoom').withId('my-room'),
    { type: 'messageAdded', message }
  )
```

The second one observes the state of the chat room fish and prints the list of messages whenever it changes:

```typescript
export const observeRoom = (pond: Pond) =>
  pond.observe(mkChatRoomFish('my-room'), msgs => {
    msgs.forEach(msg => console.log(msg))
    console.log('---')
  })
```

Running the observer on one node and the sender on two different nodes we should see the list of messages updating as we send new messages to the fish.
([Learn how to set up a swarm of multiple nodes.](/docs/os/guides/swarms))

The sequence of these messages corresponds to the sequence in which we send them from the two nodes.
Now, if we detach one of the sender nodes from the network for a bit and send some messages from it, let’s say `msg_A_1` to `msg_A_3`, while also sending `msg_B_1` to `msg_B_3` from the still connected node, we will see only `msg_B_1` to `msg_B_3` showing up at our observer.
This is because the other messages cannot be transferred right now.

When reconnecting the previously disconnected sending node, its messages `msg_A_1` to `msg_A_3` will show up at the observer after a short while.
But we notice that the messages show up not at the end of the log but interleaved with the others, for example like

```bash
    msg_B_3
    msg_B_2
    msg_B_1
    --- // ^ this was the previous state
    msg_B_3
    msg_A_3
    msg_B_2
    msg_A_2
    msg_A_1
    msg_B_1
    ---
```

How can this be?
The `onEvent` handler only ever prepends messages to the array, it never inserts them in the middle, yet we see that the state was changed “in the middle”.
The answer is that the `onEvent` handler may be invoked multiple times for the same event, if after the reception of that event other events are received that are sorted “earlier” than this event.

We recall that the current state of the fish is computed by applying one event after the other, through the `onEvent` handler.
This can be visualized like the grey zigzag line zipping together the events and their resulting states on the left-hand side of the following diagram.

![Time Travel](/images/pond/time-travel.svg)

When a new event arrives that belongs somewhere in the middle of the previously known log of events, it is inserted in its rightful spot and the current state is recomputed by applying all events again, now including the inserted event.
This is shown on the right-hand side of the diagram above; in practice the state computation starts from the state right before the inserted event in most cases, as a cache of states is kept in memory.

The remaining question is where the ordering of events comes from: how does the Pond know where a newly received events needs to be sorted into the event log?

The answer is that each event carries metadata including physical and logical timestamps and the unique identifier of the source of the event, so that an ordering can be defined that can be evaluated by all Ponds of a distributed application in the same fashion.
This allows all Ponds to sort the full event log in exactly the same order regardless of the order in which they received the events from their peers.
With this technique, it is the log itself that is eventually consistent and any business logic that deterministically computes state from such a log will be eventually consistent as well.

:::note
This explains why it is imperative that `onEvent` be fully deterministic and depend only on its inputs: the state and the event. If for example a random number generator bound to each specific edge node were used in the computation, then the same fish would compute different states on different nodes, making the system no longer eventually consistent.
:::

Equipped with this knowledge we are now ready to tackle local decision-making as explained in the next section on state
effects.
