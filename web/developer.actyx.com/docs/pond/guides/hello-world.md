---
title: Hello World
---

Your first Actyx Pond program.

As discussed in the [programming model section](../programming-model.md), the main programming unit in Actyx Pond is a _Fish_; more precisely, the code representation is wrapped up in a data type called `FishType` from which differently named fish of the same kind can be created.
A very simple fish we can write is one that just offers a friendly greeting to whoever observes it:

```typescript
const helloWorldFish = FishType.of({
  semantics: Semantics.of('ax.example.HelloWorld'),
  initialState: () => ({
    state: 'Hello world!',
  }),
  onStateChange: OnStateChange.publishPrivateState(),
})
```

:::note
All necessary imports (like `FishType`) are available from the `@actyx/pond` module.
:::

This code snippet defines a type of fish that goes by the semantics of `ax.example.HelloWorld` — this label identifies what kind of things this fish does.
Every fish has some internal — or “private” — state that it starts out with, here we provide just the friendly greeting as a string.
The final piece is technically optional; we want the greeting to be observable from the outside, so we provide a state change handler that publishes the private state for the outside to see. But how do we observe this?

```typescript
const main = async () => {
  const pond = await Pond.default()
  pond
    .observe(helloWorldFish, '')
    .do(greeting => console.log(greeting))
    .subscribe()
}
```

To bring a fish to life we need to create a pond in which it can breathe.
Once the the pond is created, we can ask the pond to observe our `helloWorldFish`; the second argument (the empty string) is the name of the particular fish of kind “hello world” that we want — in this example we don’t care about this part.
The pond will now take a look to check whether the fish we are asking for is already up and running, and it will wake it up (“hydrate” it) if that is not the case.
Once the fish is alive, we will get the initial state it publishes as an RxJS `Observable`, on which we register a callback that will print the greeting to the console and start the whole pipeline with the final `.subscribe()` call.

In this section we have seen how to create the most basic Actyx Pond program.
ActyxOS is all about event streams, so in the next section we explore how fish emit events and thus populate those event streams.
