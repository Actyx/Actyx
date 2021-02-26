---
title: Integrating a UI
hide_table_of_contents: true
---

Wrapping it up with a UI.

So far we have concentrated on the internals of a fish, on writing the business logic.
An important aspect of many apps is the involvement of human operators, which requires the presentation of a UI that they can see and interact with.

For a UI we need two things: a current state that determines what is shown, and a way to send commands to a fish to effect change.
We have seen how to observe a fish’s state in the [Hello world](/docs/pond-v1/guides/hello-world) section already, and sending commands came up when we discussed [emitting events](/docs/pond-v1/guides/events).

```typescript
export const wireUI = (pond: Pond) => {
  const sendToRoom = (message: string) =>
    pond
      .feed(chatRoomFish, 'my-room')({ type: 'addMessage', message })
      .subscribe()
  const rootEl = document.getElementById('root')

  pond
    .observe(chatRoomFish, 'my-room')
    .subscribe(state => ReactDOM.render(<Root msgs={state} sendToRoom={sendToRoom} />, rootEl))
}
```

Here we first construct a function for sending commands to the `my-room` chat room fish for later use in the UI.
Then we obtain a reference to the HTML element into which we will render the UI using the [React framework](https://reactjs.org/).
The next ingredient is the stream of state updates we receive from the chat room fish by using the `pond.observe` function.
The resulting stream of state updates is passed to the `ReactDOM.render` function to create the UI.

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
      {props.msgs.map(msg => <p key={hashCode(msg)}>{msg}</p>)}
      <input type="text" onKeyPress={handleKeyPress} />
    </div>
  )
}
```

This is a very simple Root component that we can use to visualize the chat room state:
we render a heading followed by the list of messages that was passed into the component as part of its Props, plus a text input field for entering new messages.
When the enter key is pressed in the input field, the `sendToRoom` function is invoked and the text field is cleared, ready for the next input.

The `hashCode` function is needed so that React can recognize when any of the chat messages should change — this does not occur in our code, but React cannot know that so it will complain if we don’t supply a suitable `key` property.

```typescript
const hashCode = (str: string) =>
  str
    .split('')
    .reduce((prevHash, currVal) => ((prevHash << 5) - prevHash + currVal.charCodeAt(0)) | 0, 0)
```

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

With this you have seen all important aspects of Actyx Pond in play.
You are welcome to modify the examples and experiment to your liking, and don’t hesitate to drop us a line if you have any question!
