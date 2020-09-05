---
title: Integrating a UI
---

_Wrapping it up with a UI._

So far we have concentrated on the internals of a fish, on writing the business logic.
An important aspect of many apps is the involvement of human operators, which requires the presentation of a UI that they can see and interact with.

For a UI we need two things: a current state that determines what is shown, and a way to emit events for a fish to
change its state.  We have seen how to observe a fish’s state in the [Hello world](hello-world) section already, and how
to modify it when we discussed [emitting events](state-effects).

```typescript
export const wireUI = (pond: Pond) => {
  const sendToRoom = (message: string) =>
    pond.emit(['chatRoom:my-room'], { type: 'messageAdded', message }).toPromise()
  const rootEl = document.getElementById('root')

  pond
    .observe(
      mkChatRoomFish('my-room'),
      state => ReactDOM.render(<Root msgs={state} sendToRoom={sendToRoom} />, rootEl)
    )
}
```

Here we first construct a function for emitting events to the `my-room` chat room fish for later use in the UI.
Then we obtain a reference to the HTML element into which we will render the UI using the [React framework](https://reactjs.org/).
The next ingredient is the stream of states we receive from the chat room fish by using the `pond.observe` function.
This stream of states is passed to the `ReactDOM.render` function to create the UI.

```typescript
const Root = (props: { msgs: string[]; sendToRoom: (msg: string) => void }) => {
  const handleKeyPress = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      const t = e.target as HTMLInputElement
      props.sendToRoom(t.value)
      t.value = ''
    }
  }

  return (
    <div>
      <h1>Chat Room</h1>
      {props.msgs.map((msg, idx) => <p key={`${idx}-${msg}`}>{msg}</p>)}
      <input type="text" onKeyPress={handleKeyPress} />
    </div>
  )
}
```

Note, that using a such constructed `key` for the individual React elements will probably lead to sub-optimal
performance, as React uses this key to recognize when any of the chat messages should change. A better approach would be
to add the unique `eventId` of each event to the fish's public state, and use that one instead (so instead of having a
`string[]`, we'd have a `{ message: string, id: eventId }[]`).

This is a very simple Root component that we can use to visualize the chat room state:
we render a heading followed by the list of messages that was passed into the component as part of its Props, plus a text input field for entering new messages.
When the enter key is pressed in the input field, the `sendToRoom` function is invoked and the text field is cleared, ready for the next input.

In order to try this out, we create a minimal `index.js` to tie everything together, assuming that all chat room and UI code is placed in `chat.tsx`:

```typescript
import { Pond } from 'ada'
import { wireUI } from './chat'

const main = async () => {
  const pond = await Pond.default()
  wireUI(pond)
}
main().catch(x => console.log(x))
```

## And that’s it!

<!-- TODO: link to @actyx-contrib/react-pond -->

With this you have seen all important aspects of Actyx Pond in play.
You are welcome to modify the examples and experiment to your liking, and don’t hesitate to drop us a line if you have any question!
