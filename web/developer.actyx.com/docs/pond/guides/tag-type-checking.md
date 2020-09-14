---
title: Tag Type-Checking
hide_table_of_contents: true
---

_More than you ever wanted to know about tag-associated type-checking._

In the tutorial on [tag typing](typed-tags), we have motivated the system and shown briefly how to
use it. This guide covers all the tricks, corner-cases and reasons why things are this way.

## The Objects: Tag, Tags and Where

### Where

There are objects involved with the tag query system. `Where` is the most general one,
expressing some arbitrary event selection. The field `where` of a `Fish` object accepts this
type. `Where` is the only type that can express "OR" logic:

`Tag<E1>('tag 1').or(Tag<E2>('tag 2')): Where<E1 | E2>` will match events that have `tag 1` as well as events that have
`tag 2`. Only one of the two tags has to be present! If both are present, the event is also
selected. And, as in all cases, the event may also have more tags.

### Tags

`Tags` is the next more specific one. It can be used both for selection (it implements `Where`) and
for emission.

- When used for selection, it will match events that have _all the tags_. `Tags('foo', 'bar')`
  requires _both_ `foo` and `bar` to be present on an event. (The event may have more tags than
  that and will still match.)
  
- When used for emission, all the tags are attached to the emitted event. Simple. 

`Tags('foo', 'bar')` is a shortcut for `Tag('foo').and(Tag('bar'))`.

`Tags` turn into `Where` when they are merged with `or`.

### Tag

A single tag, tied to the type of events it may be attached to. Using typescripts’s union types, we
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

## Inferred Type Requirements

### AND

Now for a closer look on how the _associated event type_ behaves when operating with tags. Say we
concatenate two tags into a set:

```typescript
const fooTag = Tag<EventFoo>('foo')

const tags = fooBarTag.and(fooTag) // type is inferred to be Tags<EventFoo>
```

`fooBarTag` may contain both: `EventFoo` and `EventBar`. But `fooTag` can _only_ contain
`EventFoo`. So `EventFoo` is the only common type between the two. What `and` does is detect this
_type intersection_ between both arguments, and narrow to it!

Logically, since `fooTag` is only attached to `EventFoo` instances, and we require `fooTag` to
be present on _all_ events, we can expect to find NO `EventBar` instances anymore, when requiring
both tags at once.

Now let’s consider both sides of the event system, producer (`Pond.emit`) and consumer (`Fish`).

- The `Fish` is required to handle all events it may possibly receive. `onEvent` must not take a
  type more narrow than the type associated with `where`. Dropping `EventBar` from the type now
  makes the implementation easier: the impossible case of receiving an `EventBar` does not have to
  be considered anymore.
  
- `Pond.emit` meanwhile must not emit events into tags that do not declare a fitting type
  association. `Pond.emit(fooTag, barEvent)` must fail. But then, `Pond.emit(fooBarTag.and(fooTag),
  barEvent)` must also fail! And it does, because `BarEvent` is dropped from the associated type.
  
### OR

The OR-case is somewhat the reverse of the AND-case. The type must be widened instead of narrowed.

```typescript
const barTag = Tag<EventBar>('bar')

const where = fooTag.or(barTag) // type is inferred to be Where<EventFoo | EventBar>
```

Logically, since we require only either of the two tags to be present on the event, we will receive
events of both associated types. A Fish running on this event set must handle both types in its
`onEvent`.

A `Where` statement cannot be emitted into. The semantics are unsound – what does it mean to emit
into one tag OR another tag? –, and the type requirement does not match: Even though the associated
type is `EventFoo | EventBar`, it’s actually incorrect to tag _either_ with both tags! `fooTag` may
not tag `EventBar`, and `barTag` may not tag `EventFoo`. (`fooTag.and(barTag)` fittingly infers
`Tag<never>` as its type, meaning it will yield no events and allow no events to be emitted,
either.)


## Types are Ultimately not Guaranteed

All this type-checking is in effect only at compile-time. Events are persistent, and possibly shared
between different programs, or versions of your program. If you emitted an `EventBar` into `fooTag`
in the past, it _will_ be passed to `onEvent` even if `where: fooTag`!

Hence you should take care to implement your `onEvent` somewhat defensively.

If you want bullet-proof safety, consider using [io-ts](https://github.com/gcanti/io-ts) for
_runtime_ type checks!


## Automatic Type Inference

This shortcut has somewhat nicer behavior with automatic type inference.
(Note that in a proper application you should _always_ statically declare your tags and their types!
Automatic type inference with

```typescript
const failsToCompile: Fish<string, MyEventType> = {
  // Expression is too complex for compiler to automatically infer Where<MyEventType>
  where: Tag('foo').and(Tag('bar')),
  
  // ... other parameters
}

const compilesFine: Fish<string, MyEventType> = {
  // Compiler infers Where<MyEventType>
  where: Tags('foo', 'bar'),
  
  // ... other parameters
}

```
