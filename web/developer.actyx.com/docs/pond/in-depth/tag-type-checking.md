---
title: Tag Type-Checking
hide_table_of_contents: true
---

_All you ever wanted to know about tag-associated type-checking._

In the [tutorial](/docs/pond/guides/typed-tags) on tag typing, we have motivated the system and shown briefly how to
use it. This guide covers all the tricks, corner-cases and design reasons.

## The Object Types: Tag, Tags and Where

### Where

There are three object types involved with the tag query system. `Where` is the most general one,
expressing some arbitrary event selection. The field `where` of a `Fish` object accepts this
type. `Where` is the only type that can express "OR" logic:

`Tag<E1>('tag 1').or(Tag<E2>('tag 2')): Where<E1 | E2>` will match events that have `tag 1` and also
events that have `tag 2`. Only one of the two tags has to be present! If both are present, the
event is also selected. And, as in all cases, the event may also have more tags.

### Tags

`Tags` is the next more specific type. It represents a "set of tags." It can be used both for
selection (it extends `Where`) and for emission.

- When used for selection, it will match events that have _all the tags_. `Tags('foo', 'bar')`
  requires _both_ `foo` and `bar` to be present on an event. (The event may have more tags than
  that and will still match.)
  
- When used for emission, all the tags are attached to the emitted event. Simple. 

`Tags('foo', 'bar')` is a shortcut for `Tag('foo').and(Tag('bar'))`.

`Tags` turn into `Where` when they are combined with `or` – `Where` is like "(at least) one of the contained sets."

#### Chaining Operators

Since `or` returns `Where`, you can _not_ do this: `(tagA.or(tagB)).and(tagC)`

You have to fall back to the normalized form: `(tagA.and(tagC)).or(tagB.and(tagC))`

We do not support the first version, in order to guard against mistakes of the following kind: Accidentally
writing `tagA.or(tagB).and(tagC)` instead of `tagA.or(tagB.and(tagC))` – just one misplaced bracket
and the whole query is significantly altered, perhaps selecting nothing at all, because tagA and
tagC don’t overlap. (According to normal boolean logic rules, one might expect the `and` to bind
stronger than the `or`, but this is very hard to mimic in TS evaluation order where `or` will
always be called first!)

### Tag

A single tag, tied to the type of events it may be attached to. Using TypeScript’s union types, we
are unrestricted in the number of associated events:

```typescript
type EventFoo = {
  type: 'foo',
  someNumber: number,
}

type EventBar = {
  type: 'bar',
  someString: string
}

// Tagged union
type Event = EventFoo | EventBar

const fooBarTag = Tag<Event>('foo-or-bar-event')
```

A single tag may turn into `Tags` in two ways.

- Calling `withId`, we create a set of tags that contains both the original tag, as well as a
  postfixed version: `fooBarTag.withId('id')` gives `Tags('foo-or-bar-event',
  'foo-or-bar-event:id')`. This is so that the set of _all_ foo-bar-events remains selectable via
  the general tag. There are no prefix-searches on tags – they can only be matched exactly. So we
  must attach both.
  
- Calling `and` with more tags, we simply create a `Tags` object containing the concatenation.

:::tip Inspect your Queries
Since Pond 2.2 you can call `toString()` on `Where` / `Tags` / `Tag` objects to find out what your query does
under the hood, at a glance.
:::

## Inferred Type Requirements

### AND

Now for a closer look on how the _associated event type_ behaves when operating with tags. Say we
concatenate two tags into a set:

```typescript
const fooTag = Tag<EventFoo>('foo')

const tags = fooBarTag.and(fooTag) // type is inferred to be Tags<EventFoo>
```

`fooBarTag` may contain both: `EventFoo` and `EventBar`. But `fooTag` can _only_ contain
`EventFoo`. So `EventFoo` is the only common type between the two. `and` does detect this
_type intersection_ between both arguments, and narrow to it!

Logically, since `fooTag` is only attached to `EventFoo` instances, and we require `fooTag` to
be present on _all_ events, we can expect to find _no_ `EventBar` instances anymore, when requiring
both tags at once.

Now let’s consider both sides of the event system, producer (`Pond.emit`) and consumer (`Fish`).

- The Fish is required to handle all events it may possibly receive. `onEvent` must not take a
  type more narrow than the type associated with `where`. Dropping `EventBar` from the type now
  makes the implementation easier: the impossible case of receiving an `EventBar` does not have to
  be considered anymore by `onEvent`.
  
- Events passed into`Pond.emit` must not be tagged with tags that do not declare a fitting type
  association. Emitting a `BarEvent` with a `fooTag` or `fooBarTag.and(fooTag)` fails because
  `BarEvent` is dropped from the associated type.
  
### OR

The OR-case is somewhat the reverse of the AND-case. The type is widened instead of narrowed.

```typescript
const barTag = Tag<EventBar>('bar')

// type is inferred to be Where<EventFoo | EventBar>
const where = fooTag.or(barTag)
```

Logically, since we require only either of the two tags to be present on the event, we will receive
events of both associated types. A Fish running on this event set must handle both types in its
`onEvent`.

A `Where` statement cannot be emitted into. The intent is unclear – what does it mean to emit into
one tag OR another tag? –, and the type requirement does not match: Even though the associated type
is `EventFoo | EventBar`, it’s actually incorrect to tag of the events _either_ with both tags!
`fooTag` may not tag `EventBar`, and `barTag` may not tag `EventFoo`.

(`fooTag.and(barTag)` fittingly infers `Tag<never>` in this case, meaning it will yield no events and
allow no events to be emitted, either.)

## Types are Ultimately not Guaranteed

All this type-checking is in effect only at compile-time. Events are persistent, and possibly shared
between different programs, or versions of your program. If you emitted an `EventBar` into `fooTag`
in the past, it _will_ be passed to `onEvent` even if `where: fooTag`!

Hence you should take care to implement your `onEvent` somewhat defensively. Below, we outline some
good architecture practices to keep you safe.

In any case, if you want bullet-proof safety, consider using [io-ts](https://github.com/gcanti/io-ts) for
_runtime_ type checks!

## Architecture, and Changing the Event Schema

It is strongly recommended you declare each of your tags only _once_: statically, with fixed associated
types. And then always reference the canonical instance. That is:
```typescript
// Somewhere close to the definition of the event types:
const fooBarTag = Tag<EventFoo | EventBar>('foo-or-bar-event')

// Then do this:
pond.emit(fooBarTag, someFooEvent)

// Rather than this:
pond.emit(Tag('foo-or-bar-event'), someFooEvent)
```

When changing your application, you should then take care to never _narrow_ an associated
type – because potentially there are old persisted events of the associated type you are
removing. Consumers must stay aware of that.

As an example, let us assume we want to incompatibly change the shape of `EventFoo`:

```typescript
// Old shape
type EventFoo = {
  type: 'foo',
  someNumber: number,
}

// Desired new shape
type EventFoo = {
  type: // ?
  
  data: {
	mark: number
	details: string
  }
}
```

Keep in mind that no matter how thoroughly you try to purge `EventFoo` from your application code,
instances may be persisted in your ActyxOS swarm! So we recommend instead your application code does
not forget about `EventFoo` at all.

```typescript
// Desired new shape
type EventFooV2 = {
  type: 'foo.v2'
  
  data: {
	mark: number
	details: string
  }
}

// Extend the tag, but do not forget about the original EventFoo shape
const fooBarTag = Tag<EventFoo | EventBar | EventFooV2>('foo-or-bar-event')
```

Obviously, `fooBarTag` will still allow emission of `EventFoo`. To prevent unwitting producers from
doing this, you should design your module something like this:

```typescript
// This type can only be used for subscribing, not for emission
export const FooBarEvents: Where<EventFoo | EventBar | EventFooV2> = fooBarTag

// For producers of FooEvent2
export const emitFooEvent = (pond: Pond, mark: number, details: string) => {
	const fooEvt: EventFoo2 = { type: 'foo.v2', data: { mark, details } }
    return pond.emit(fooBarTag, fooEvt)
}

// And stop exporting any old emitFooEvent function you may have offered
```

If you want to spare new consumers the burden of supporting the old `EventFoo`, and those consumers
do not care about missing out on the old data, consider introducing a new tag, as well:


```typescript
// For consumers that care about old events
export const FooBarEvents: Where<EventFoo | EventBar | EventFooV2> = fooBarTag

const fooBarTagV2 = Tag<EventBar | EventFooV2>('foo-or-bar-event.v2')

// For new consumers that do not care
export const FooBarEventsV2: Where<EventBar | EventFooV2> = fooBarTagV2

// For emission
const backwardsCompatTags = fooBarTag.and(fooBarTagV2)

export const emitFooEvent = (pond: Pond, mark: number, details: string) => {
	const fooEvt: EventFoo2 = { type: 'foo.v2', data: { mark, details } }
    return pond.emit(backwardsCompatTags, fooEvt)
}

// And stop exporting any old emitFooEvent function you may have offered
```

## Automatic Type Inference

When giving tags inline, types can often be automatically inferred. This is nice for
prototyping. Beware though that it’s the exact reverse of static type-safety guarantees. The
compiler simply trusts you that things will work out.

```typescript
const compilesFine: Fish<string, MyEventType> = {
  // Compiler infers Where<MyEventType>
  where: Tags('foo', 'bar'),
  
  // ... other parameters
}

const failsToCompile: Fish<string, MyEventType> = {
  // If using operators, type inference will fail;
  // you could cast to the desired type manually: `... as Where<MyEventType>`
  where: Tag('foo').and(Tag('bar')),
  
  // ... other parameters
}
```
