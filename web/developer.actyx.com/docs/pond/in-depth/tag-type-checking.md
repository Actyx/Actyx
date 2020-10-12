---
title: Tag Type-Checking
---

_All you ever wanted to know about tag-associated type-checking._

In the [tutorial](/docs/pond/guides/typed-tags) on tag typing, we have motivated the system and shown briefly how to
use it. This guide covers all the tricks, corner-cases and design reasons.

## The Object Types: Tag, Tags and Where

There are three object types involved with the tag query system. They form a hierarchy: `Tag` is the
most specific one, `Tags` is a like set of `Tag` objects, and `Where` is like a collection of `Tags` objects.

### Tag

A single tag, tied to the type of events it may be attached to. Using TypeScript’s union types, we
are unrestricted in the number of associated events:

```typescript
enum EventType {
  foo = 'foo',
  bar = 'bar',
}

type EventFoo = {
  eventType: EventType.foo,
  someNumber: number,
}

type EventBar = {
  eventType: EventType.bar,
  someString: string
}

// Union type
type Event = EventFoo | EventBar

const fooBarTag = Tag<Event>('foo-or-bar-event')
```

A `Tag` can be used both for emission and for event selection:

```typescript
pond.emit(fooBarTag, { eventType: EventType.foo, someNumber: 5 })

const lastSeenType: Fish<EventType | undefined, Event> = {
  // Select all events with this Tag
  where: fooBarTag,
  
  initialState: undefined,
  
  onEvent: (_state, event) => event.eventType,
  
  fishId: FishId.of('last-seen', 'foo-bar', 1),
}
```

### Tags

`Tags` is the next more general type. It represents a "set of tags."

A single tag may turn into `Tags` in two ways.

- Calling `withId`, we create a set of tags that contains both the original tag, as well as a
  postfixed version: `fooBarTag.withId('id')` gives `Tags('foo-or-bar-event',
  'foo-or-bar-event:id')`. This is so that the set of _all_ foo-bar-events remains selectable via
  the general tag. There are no prefix-searches on tags – they can only be matched exactly. So we
  must attach both.
  
- Calling `and` gives a `Tags` object containing them all: `Tag('a').and(Tag('b'))` is equivalent to
  `Tags('a', 'b')`

A `Tags` object can be used both for emission and for event selection, just like a single `Tag`.

- When used for selection, it will match events that have _all the tags_. `Tags('foo', 'bar')`
  requires _both_ `foo` and `bar` to be present on an event. (The event may have more tags than
  that and will still match.)
  
- When used for emission, all the tags are attached to the emitted event.

```typescript
const FooTag = Tag<EventFoo>('foo')

const CountingTag = Tag<any>('count-this-event')

pond.emit(FooTag.and(CountingTag), { eventType: EventType.foo, someNumber: 5 })

const countingFoos: Fish<number, Event> = {
  // Select all events with both 'foo' and 'count-this-event'
  where: FooTag.and(CountingTag),
  
  initialState: 0,
  
  onEvent: (state, _event) => state + 1,
  
  fishId: FishId.of('counting', 'foos', 1),
}
```

### Where

`Where` is the most general type, expressing some arbitrary event selection. The field `where` of a
Fish object accepts this type, and hence accepts `Tag` and `Tags` as well, since they are extensions
of `Where`.

`Tag<E1>('tag 1').or(Tag<E2>('tag 2')): Where<E1 | E2>` will match events that have `tag 1` and also
events that have `tag 2`. It suffices if one of both tags is present. If both are present, the
event is also selected. And, as in all cases, the event may also have more tags.

:::tip Inspect your Queries

Since Pond 2.2 you can call `toString()` on `Where` / `Tags` / `Tag` objects to find out what your query does
under the hood, at a glance.

:::

#### Chaining Operators

Since `or` returns `Where`, you can _not_ do this: `(tagA.or(tagB)).and(tagC)`

You have to fall back to the normalized form: `(tagA.and(tagC)).or(tagB.and(tagC))`

We do not support the first version, in order to guard against mistakes of the following kind:
Accidentally writing `tagA.or(tagB).and(tagC)` instead of `tagA.or(tagB.and(tagC))` – just one
misplaced bracket and the whole query is significantly altered, perhaps selecting nothing at all,
because events with tagA and events with tagC don’t overlap. (According to normal boolean logic
rules, one might expect the `and` to bind stronger than the `or`, but this is very hard to mimic in
TypeScript evaluation order where `or` will always be called first!)

## Inferred Type Requirements

### AND

Now for a closer look on how the _associated event type_ behaves when operating with tags. Say we
concatenate two tags into a set:

```typescript
const fooBarTag = Tag<EventFoo | EventBar>('foo-or-bar-event')
const fooTag = Tag<EventFoo>('foo')

const tags = fooBarTag.and(fooTag) // type is inferred to be Tags<EventFoo>
```

`fooBarTag` may contain both: `EventFoo` and `EventBar`. But `fooTag` can _only_ contain
`EventFoo`. So `EventFoo` is the only common type between the two. `and` does detect this
_type intersection_ between both arguments, and narrows down the result’s associated type accordingly.

Logically, since `fooTag` is only attached to `EventFoo` instances, and we require `fooTag` to be
present on the selected events, we can expect to find _no_ `EventBar` instances anymore, when
requiring both tags at once.

Now let’s consider both sides of the event system, producer (`Pond.emit`) and consumer (`Fish`).

- The Fish is required to handle all events it may possibly receive. `onEvent` must not take a
  type more narrow than the type associated with `where`. Dropping `EventBar` from the type now
  makes the implementation easier: the impossible case of receiving an `EventBar` does not have to
  be considered anymore by `onEvent`.
  
- Events passed into`Pond.emit` must not be tagged with tags that do not declare a fitting type
  association. Emitting a `BarEvent` with a `fooTag` or `fooBarTag.and(fooTag)` fails because
  `BarEvent` has been dropped from the associated type.
  
### OR

The OR-case is somewhat the reverse of the AND-case. The type is widened instead of narrowed.

```typescript
const fooTag = Tag<EventFoo>('foo')
const barTag = Tag<EventBar>('bar')

// type is inferred to be Where<EventFoo | EventBar>
const where = fooTag.or(barTag)
```

Logically, since we require only either of the two tags to be present on the event, we will receive
events of both associated types. A Fish running on this event set must handle both types in its
`onEvent`.

A `Where` statement cannot be used for emission. The intent is unclear: What does it mean to emit
an event tagged with one tag OR another tag? And also the type requirement does not match: Even though the
associated type is `EventFoo | EventBar`, it’s actually incorrect to tag _either_ of the events with
both tags!  `fooTag` may not tag `EventBar`, and `barTag` may not tag `EventFoo`.

(`fooTag.and(barTag)` fittingly infers `Tag<never>` in this case, meaning it will yield no events and
allow no events to be emitted, either.)

## Types are Ultimately not Guaranteed

All this type-checking is in effect only at compile-time. Events are persistent, and possibly shared
between different programs, or versions of your program. If you emitted an `EventBar` with `fooTag`
attached in the past, it _will_ be passed to `onEvent`, if `where: fooTag`.

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
instances may be persisted in your ActyxOS swarm! So we recommend that your application code does
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
    const fooEvt: EventFooV2 = { type: 'foo.v2', data: { mark, details } }
    return pond.emit(fooBarTag, fooEvt)
}

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

```

Finally, stop exporting any old `emitFooEvent` function you may have offered.

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
