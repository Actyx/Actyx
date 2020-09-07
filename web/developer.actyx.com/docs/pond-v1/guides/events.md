---
title: Events
hide_table_of_contents: true
---

A Fish is a source of events, so let’s emit some events!

Each fish — identified by its FishType (i.e. semantics) and name — emits exactly one event stream on each node that it runs on; that event stream has the very same name as the fish plus the node’s source ID.
This means that a fish cannot emit events into another fish’s stream, each fish has its own event stream.

Another important point is that a fish does not act on its own, the fish code only runs when external triggers kick it into action.
In the previous section we have seen that observing a fish wakes it up, making it publish its initial state.
The trigger we need for emitting events is a _command_, someone needs to tell the fish to do something.

Therefore, the fish definition includes an `onCommand` handler that can be invoked from the outside, and from this handler events can be emitted.
The FishType also declares which type of events this kind of fish emits, so before we can write the code that emits the events we will need to define what they shall look like.

```typescript
type ChatRoomCommand = { type: 'addMessage', message: string }
type ChatRoomEvent = { type: 'messageAdded', message: string }
```

In this example we consider a fish that models a chat room. The main action that can be performed on such a room is to add a new message, so we define a command for this purpose. The `onCommand` handler will then transform any such incoming command into a matching event, with the straight-forward definition given above.

```typescript
const chatRoomOnCommand: OnCommand<{}, ChatRoomCommand, ChatRoomEvent> = (_state, command) => {
  switch (command.type) {
    case 'addMessage': {
      const { message } = command
      return [{ type: 'messageAdded', message }]
    }
    default:
      return unreachableOrElse(command.type, [])
  }
}
```

:::note
All necessary imports (like `OnCommand`) are available from the `@actyx/pond` module.
:::

The definition of an `onCommand` handler starts by declaring the types of state, commands, and events that this handler will process.
The state is passed in as first function argument, but we don’t need it in this example and use the empty object — this will become important in the section on [command validation](commands) later on.

A typical fish will handle multiple commands, so we handle the command type in a switch statement; the default case is using a helper function from the `ada` module that guards against forgetting to handle any of the cases — you can try to remove the first case and observe the resulting compiler error.
The return value of `onCommand` is a list of events to be emitted.
In the case of an `addMessage` command, we extract the message and create a `messageAdded` event from it.
It is good practice to name all commands such that they contain a verb in imperative form, and all events as a minimal sentence in past tense, reflecting that an event is a fact we know about the past.

Now all that remains is to tie this together with a fish definition:

```typescript
const chatRoomFish = FishType.of({
  semantics: Semantics.of('ax.example.ChatRoom'),
  initialState: () => ({
    state: {}
  }),
  onCommand: chatRoomOnCommand
})
```

With this, we can send commands to this fish using a main program that uses the `Pond.feed()` method:

```typescript
export const main2 = (pond: Pond) =>
  pond.feed(chatRoomFish, 'my-room')({ type: 'addMessage', message: 'hello' }).toPromise()
```

Here, we select a fish with a more interesting name than in the previous example: this fish represents a specific chat room named `my-room`.
The `feed` function returns a function for feeding that fish, which we immediately call with a command to add the `hello` message to the chat room.
Sending a command to a fish is an asynchronous operation, hence the return value of feeding a fish is an `Observable<void>` that is completed when the command has been accepted by the fish.
RxJS `Observable` does not actually do anything unless the pipeline is started, in this case by calling `.toPromise()` on it.
Returning this promise allows the caller of this procedure to wait until the command is successfully sent.

Running this minimal Actyx Pond program will wake up the chat room fish and send it a single command that will trigger the fish to emit a single event.
In the next section we look into making use of these events to build up the internal state of the fish.
