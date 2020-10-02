---
title: "Designing the C# Pond: Union Types"
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [Actyx Pond, CSharp, C#]
---

One of the many projects we’re currently pushing forward at Actyx is a port of the [Actyx
Pond V2](./2020-07-24-pond-v2-release) from TypeScript to C#.

C# and TypeScript build on very different foundations. Both are modern multi-paradigm languages;
both have somewhat dynamic function dispatch mechanisms; but the typical C# program is still very much
concerned with the _runtime type_ of objects, as modelled by the CLR. TypeScript meanwhile is
all about _type shapes_ (or duck typing): The type system itself is quite strong, but its _reality_ does
not carry over into the (JavaScript) runtime.

One way this difference in typing plays out is _union types_. Union types are a cornerstone of TypeScript
programming, and prominently feature in our TypeScript Pond interfaces. But C# does not have an exact
equivalent. In this blog post we are looking at ways to preserve all the TypeScript Pond’s features, without
giving up idiomatic C#.

<!-- truncate -->

The C# equivalent of unioning any two types is clunky: An `Either<A, B>` type, no matter how
`Either` is implemented, does not automatically cover values of type `A`. Contrary to TypeScript,
values of type `A` or `B` would have to be explicitly wrapped into `Either<A, B>`.

The actually idiomatic alternative to union types in C# is to just use a common interface among all
types of the union. But in a producer/consumer architecture, this approach is problematic: Every
new consumer would have to change code among all event producers, adding "its own" union
interface. An event read by five different consumers would end up implementing five different union
interfaces. (Or its definition would be copied five times.)

That architecture _does_ have advantages – e.g. it’s easy to see who the consumers are at a
glance – but we do not want to make it mandatory.

So we are about to do a very simple thing. Rather than having the code specify just one subscription and
one event handler ("onEvent") per Fish, the Fish may have multiple selections, each with its own event
handler. It’s just like a union, only the real union is never explicitly constructed.

```cs
new FishBuilder(fishId, initialState)
  .subscribeTo<E>(eventSelector1, handlerForE)
  .subscribeTo<F>(eventSelector2, handlerForF)
  .build()
```

Imagine here `eventSelector1` to be some typed selector of events, where all contained events have type
`E`. `handlerForE` then is a function `S onEvent(S oldState, E event);`, much like [`onEvent`](https://developer.actyx.com/docs/pond/guides/local-state) in the
TypeScript Pond. In the next line, events of type `F` are selected, and the `handlerForF` takes `F event`.

## Ways of implementing handlers

If you are serious about object-oriented design, you will probably put very little code in the
handler function itself. Instead, you would view the update logic as either a method of the event,
or a method of the state.

### Seeing the Event as responsible for updating

Let’s see how the handler would be implemented when update logic is put into the event definition.

```cs
interface IMyEvent
{
    MyState updateState(MyState oldState);
}

// Later:
new FishBuilder(fishId, initialState)
  .subscribeTo<IMyEvent>(mySelection, (oldState, event) => event.updateState(oldState))
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
  .subscribeTo<ISomeForeignEvent>(
    someForeignEventsSelection,
    (state, event) => state.consumeSomeForeignEvent(event)
  )
  .build()
```

If `ISomeForeignEvent` covers different concrete types, `updateWith` may have to use `instanceof`
checks to find out what to do. (Or, more elegantly, a `switch` on the input.) In turn, the
producer’s code does not have to be touched: All logic lives on our side, the consumer’s side.

### Shortcuts

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
    FishBuilder<S> subscribeTo<E>(Selection<E> selection, EventHandler<S, E> handler);

    // If event type implements logic to update S, we don’t need an explicit handler!
    FishBuilder<S> subscribeTo<E>(Selection<E> selection) where E : IUpdateState<S>;
}
```

We would like to define the case where logic lives inside `S` in an analogous manner. Unfortunately,
C# is not that expressive yet.

```cs
class FishBuilder<S>
{
    // Fails to compile, because generic S is not scoped to the method
    FishBuilder<S> subscribeTo<E>(Selection<E> selection) where S : IUpdatedBy<E>;
}

// Works, but captures only **one** E, even if S supports multiple different E.
class FishBuilder<S> where S : IUpdatedBy<E>
{
    FishBuilder<S> subscribeTo<E>(Selection<E> selection)
}
```

So let’s capture `S` and `E` at the same time:

```csharp
FishBuilder.for(fishId, initialState, events1, events2, events3)
    .subscribeTo(events4, handler)
    .build();

// The implementation is not so nice:
static FishBuilder<S> for<S, E, F, G>(
    FishId fishId,
    S initialState,
    Selection<E> selection1,
    Selection<F> selection2,
    Selection<G> selection3,
) where S : IUpdatedBy<E>, IUpdatedBy<F>, IUpdatedBy<G>;
```

Basically, we must offer a different impl. per number of subscriptions. That’s okay, a lot of
libraries solve similar problems the same way.

### Future Work

Attributes in C# can make lots of things very easy to write down. One might imagine an attribute for
handler declaration. Perhaps it might even include the selection of events.

```cs
class MyState
{
  [ActyxEvtHandler(SomeEvent.Class, Where("some-event-selector"))]
  MyState updateWith(SomeEvent evt) { /* ... */ };
}
```

However, compile-time and runtime-checks are starting to mix in this approach. Likely it won’t reach
maximal compile-time safety. We will focus on shipping the slightly more verbose APIs first, since
everything else must be based on them in any case. Then we will look into how Attributes can improve
ease of use.

### Closing Words

And that’s it for now. Should you have any wishes or suggestions for our upcoming C# libraries, [contact us](mailto:developer@actyx.io)!
