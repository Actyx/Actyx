---
title: Hello World
---

_Your first Actyx Pond program._

The main programming unit in Actyx Pond is a _Fish_.
A very simple fish we can write is one that just offers a friendly greeting to whoever observes it:

```typescript
const helloWorldFish = {
  fishId: FishId.of('ax.example.HelloWorld', 'getting-started', 0),
  initialState: 'Hello world!',
  onEvent: (s: string, _event: any, _metadata: Metadata) => s,
  where: noEvents,
}
```

:::note
All necessary imports (like `Tags`) are available from the `@actyx/pond` module.
:::

This code snippet defines a type of fish that goes by the fish type of `ax.example.HelloWorld` — this label identifies
what kind of things this fish does.  Every fish has some state that it starts out with, here we provide just the
friendly greeting as a string.  Further, we need to provide an `onEvent` handler, in this case, we're just ignoring all
incoming events – the state is never changed.  At last, we're providing a query describing which events this fish is
interested in, which in this case is none.

Now, let's see how the friendly fish can give us a warm greeting:

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

You should see the greeting logged to the console.
:::

To bring a fish to life we need to create a pond in which it can breathe.
Once the pond is created, we can ask the pond to observe our `helloWorldFish`; the second argument is a callback, which
is called with every state update of the fish.

The pond will now take a look to check whether the fish we are asking for is already up and running, and it will wake it
up (“hydrate” it) if that is not the case.  Once the fish is alive, our callback is called with the initial state the
fish publishes.

In this section we have seen how to create the most basic Actyx Pond program.
ActyxOS is all about event streams, so in the next section we explore how fish emit events and thus populate those event streams.
