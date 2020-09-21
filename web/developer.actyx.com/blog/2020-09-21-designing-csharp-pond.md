---
title: A Farewell to Union Types
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [Actyx Pond Csharp C#]
---

One of the many projects we’re pushing forward in Actyx currently is an implementation of the [Actyx
Pond V2](./2020-07-24-pond-v2-release) in C#.

C# and TypeScript build on quite different foundations. Both are modern multi-paradigm languages;
both have somewhat dynamic function dispatch mechanisms; but the typical C# program is still very much
concerned with the _runtime type_ of objects, as modelled by the CLR. TypeScript meanwhile is
all about _type shapes_ (or duck typing): The type system itself quite strong, but its _reality_ is
not really rooted in the runtime.

One way this difference in typing plays out is "union types." Union types are a cornerstone of TS
programming, and prominently feature in our TS Pond interfaces. But C# does not have an exact
equivalent. In this blog post we are looking at ways to still express the same interface in C# as we
do in TS.

<!-- truncate -->

The C# equivalent of unioning any two types is clunky: An `Either<A, B>` type, no matter how
`Either` is implemented, does not automatically cover values of type `A`. Contrary to TS, such
values would have to be explicitly wrapped.

The actually idiomatic alternative to union types in C# is to just use a common interface among all
types of the union. But in a producer/consumer architecture, such an approach is unfortunate: Every
new consumer would have to change code among all event producers, adding "its own" union
interface. An event read by five different consumers would end up implementing five different union
interfaces.

Such an architecture _does_ have advantages – e.g. it’s easy to see who the consumers are at a
glance – but we do not want to make it mandatory.

So we are about to do a very simple thing. Rather than having you specify just one subscription and
one event handler ("onEvent") in a Fish, it may have multiple subscriptions, each with its own event
handler. It’s just like a union, only the real union is never explicitly constructed.

```csharp
new FishBuilder(fishId, initialState)
  .subscribeTo<E>(events1, handlerForE)
  .subscribeTo<F>(events2, handlerForF)
  .build()
```

Imagine here `events1` to be some typed selector of events, where all contained events have type
`E`. `handlerForE` then is a function `S onEvent(S oldState, E event);`, much like `onEvent` in the
TS Pond. In the next line, events of type `F` are selected, and the `handlerForF` takes `F event`.

## Ways of implementing handlers

If you are serious about object-oriented design, you will probably put very little code in the
handler function itself. Instead, you would view the update logic as either a method of the event,
or a method of the state.

### Seeing the Event as responsible for updating

Let’s see how the handler would be implemented when update logic is put into the event objects:

```cs
interface IMyEvent
{
    MyState updateState(MyState oldState);
}

// Later:
new FishBuilder(fishId, initialState)
  .subscribeTo<IMyEvent>(myEvents, (oldState, event) => event.updateState(oldState))
  .build()
```

`IMyEvent` may actually cover more than one concrete class, using
[JsonSubTypes](https://github.com/manuc66/JsonSubTypes).

This is a very nice approach if `IMyEvent` is owned by the same code module as the Fish: We are fine
with tight coupling. But if the producer lives in a different module, we are back to the problem of
_having to go there_ and add an additional interface implementation on the event type.

### Seeing the State as responsible for updating

So instead we may see "being updated" as the state’s responsibility:

```cs
class MyState
{
  // Among other things:
  MyState consumeSomeForeignEvent(ISomeForeignEvent event);
}

// Later:
new FishBuilder(fishId, initialState)
  .subscribeTo<ISomeForeignEvent>(someForeignEvents, (state, event) => state.consumeSomeForeignEvent(event))
  .build()
```

If `ISomeForeignEvent` covers different concrete types, `updateWith` may have to use `instanceof`
checks to find out what to do. In turn, the producer’s code does not have to be touched: All logic
lives on our side, the consumer’s side.

### Making it nicer

It’s very simple to add a shortcut for the case where we put the logic into events:

```cs
interface EventHandler<S, in E>
{
    S onEvent(S oldState, E eventPayload);
}

interface IUpdateState<S>
{
    S updateState(S oldState);
}

class FishBuilder<S>
{
    // Explicitly specify handler
    FishBuilder<S> subscribeTo<E>(Tags<E> subscription, EventHandler<S, E> handler);

    // If event type implements logic to update S, we don’t need an explicit handler!
    FishBuilder<S> subscribeTo<E>(Tags<E> subscription) where E : IUpdateState<S>;
}
```

However, C# does not like the analogous way of having the logic inside state:

```cs

interface IUpdatedBy<in E>
{
    void updateWith(E eventPayload);
}

class FishBuilder<S>
{
    // Fails to compile, because generic E is only in the method signature
    FishBuilder<S> subscribeTo<E>(Tags<E> subscription) where S : IUpdatedBy<E>;
}
```

Perhaps we can find a way around it, or perhaps it does not matter much.
