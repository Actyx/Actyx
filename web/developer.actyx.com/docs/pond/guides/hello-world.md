---
title: Hello World
hide_table_of_contents: true
---

_Your first Actyx Pond program._

The main programming unit in Actyx Pond is a _Fish_. A fish is like a living entity: It
"feeds on" little pieces information – events – produced by connected ActyxOS nodes. From the
events, it builds its _state_. So when a new event is produced somewhere, the state updates. In your
application, you can _observe_ a fish’s state meaning you get realtime updates!

Let’s start with a very simple fish.

```typescript
import { FishId, Metadata, allEvents, noEvents } from '@actyx/pond'

const helloWorldFish = {
  fishId: FishId.of('HelloWorld-Example', 'getting-started', 0), // For caching
  initialState: 'Hello world!',
  onEvent: (_oldState: string, event: any, _metadata: Metadata) => event,
  where: allEvents,
}
```

The `onEvent` handler we provide takes the existing state and an event, in order to produce an
updated state. It’s like `Array.reduce`. In this case, we just set the state to the latest event we
have seen! Of course, this could be much more elaborate.

The `initialState` defines what state we are in before having seen any events. Fittingly, we just
send out a general greeting: Hello, world!

Finally, we're providing a `where` clause describing which events this fish is interested in. By
passing `allEvents` we do in fact select all events created by the application; we might also pass
`noEvents` to select process no events and always stay at the initial state. But of course, there is
rich functionality for selecting only _some_ events. We will get to that later.

For now, let's see how the friendly fish can give us a warm greeting:

```typescript
const main = async () => {
  const pond = await Pond.default()

  pond.observe(helloWorldFish, console.log)
}
```

:::note Try it out!

```text
git clone https://github.com/Actyx/quickstart.git
cd quickstart/hello-world
npm install
npm start
```

You should see the greeting logged to the console – unless your swarm already has some events, in
which case you will see the latest one!
:::

To bring a fish to life we need to create a pond in which it can breathe.
Once the pond is created, we can ask the pond to observe our `helloWorldFish`; the second argument is a callback, which
is called with every state update of the fish.

In this section we have seen how to create the most basic Actyx Pond program.
ActyxOS is all about event streams, so in the next section we explore how events are mitted and thus populate those event streams.
