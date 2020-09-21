---
title: Designing the Pond in C# - A Farewell to Union Types
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
concerned with the _concrete type_ of objects, as modelled by the CLR. TypeScript meanwhile is
all about _type shapes_ (or duck typing): The type system itself quite strong, but its _reality_ is
not as deeply rooted in the runtime.

In this blog post, we are going to explore briefly the challenges this has raised for us in porting
the Pond, and how we are planning to overcome them to the effect of perhaps an API that’s actually
nicer than the TypeScript one.

<!-- truncate -->

An important TypeScript feature used by the Pond are _union types_. A Fish declares a subscription
set encompassing any number of event types, then implements an `onEvent` function that accepts the
union of those types as parameter. One handler for everything.

In TS this "union" is formed simply via e.g. `type TypeAorB = TypeA | TypeB`, expressing that anything of
type `TypeAorB` may be _either_ of `TypeA` or of `TypeB`. Any object that is of `TypeA` is
_automatically_ also of `TypeAorB` as defined here!

Consider the brute-force equivalent in C#: `Either<TypeA, TypeB>`. Regardless of the implementation
of `Either`, an object of `TypeA` will never automatically be `Either<TypeA, TypeB>` as well.

How does the TS programmer tell whether an input of `TypeAorB` is actually TypeA, or actually TypeB?
Since none of these types actually has a _runtime reality_ to it, `instanceof` is not available.

Instead, a concept known as "tagged unions" must be employed: A common field is introduced between
all relevant types, and assigned a different singleton-type (the "tag") for each type.
```ts
type TypeA = {
  // Read: Field 'discriminant' may only ever contain the string 'A' (tag for this type)
  discriminant: 'A'
  
  // This field meanwhile may contain *any* string
  someDataField: string
  
  // .. more data fields ..
}

type TypeB = {
  discriminant: 'B'
  
  // ... data fields ...
}
```
As soon as the code has asserted `obj.discriminant === 'A'`, TypeScript will allow the object to be
used as if it was `TypeA`. Actually there are still no guarantees: All other fields may be
missing. The producer of `obj` may have misbehaved and constructed a `TypeB`, only with the
`discriminant` set to A. The compile-time checks of TS forbid this, but the data may come from a
plain JS producer, or from JSON...

JSON handling consequently is another point where we must depart from our TypeScript
ways. Deserializing an object in TS just gives `any`, which you may cast as you wish. So what we are
doing for event deserialization is ultimately trust the user’s type declarations all the way: There
are no checks performed on the JSON we read, as to it really "has" the right type. This works fine
as long as everything is read and written by parts of the same code-base.

In C#, deserialization must declare the type to deserialize _into_ upfront, and depending on the
configuration it may then even fail when fields are missing, etc.

### Handlers per Type

So what we will be doing is to depart from the "one event handler for everything" model. The large
`switch` statements in leads to in some cases have always been bogus, actually.
