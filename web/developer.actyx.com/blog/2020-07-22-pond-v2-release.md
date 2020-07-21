---
title: Introducing Pond Version 2
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [Actyx Pond]
---

We are pleased to announce the release of the Actyx Pond Version 2. [Download from npm](LINKPLS)

Read on for a brief overview of the changes, which we have developed with the goal of reducing
boilerplate and allowing more flexibility and elegance in your application architecture.

<!-- truncate -->

## Tags

The biggest change we’re rolling out to the Pond and the ActyxOS EventService in general with
this version is that Events are now indexed based on Tags assigned by your application. There can be
any number of Tags given for an Event. That means an Event no longer belongs to one single stream
identified by _semantics_ and _fishName_, but instead can belong to many streams – each identified by just a
string, as Tags are nothing but strings.

To retrieve Events based on their Tags, you can then employ logic like:
- Events with Tag 'foo'
- Events with Tag 'foo' or Tag 'bar' (or both)
- Events with both Tags 'foo' and 'bar'

Additional Tags are always O.K., so if an Event has Tags `['foo', 'bar', 'baz']` it would also
match, in all three cases.

For the Pond, we are shipping multiple nice mechanisms for expressing your Tag-based queries.

A very quick demonstration ([detailed docs](LINKPLS)):

```typescript
// Select events with either both tag0 and tag1, or just tag2.
// (Events with all three tags also match.)
const where = TagQuery.matchAnyOf(
  'tag2',
  TagQuery.requireAll('tag0', 'tag1'),
)


// Alternative, declare your tags in association with event types:
const Tag0 = Tag<Type0>('tag0')
const Tag1 = Tag<Type1>('tag1')
const Tag2 = Tag<Type2>('tag2')

const where = tag2.or(tag0.and(tag1))

```

Also be sure to check out [our guide on how to design your application architecture based on tags](LINKPLS).

## Direct Event Emission

In version 1 of the Actyx Pond, all Events had to be emitted by Fish, from a received Command.
Now, Events can be emitted freely without any Fish at hand.
```typescript
pond.emit(['myFirstTag', 'mySecondTag'], myEventPayload)
```

It is still recommended that you organize ownership of Events (by type) into modules, for example:

```typescript
import { getUserTags } from './user-fish'

type MaterialConsumed = // The type you have designed

// Union of all types related to material
type MaterialEvent = MaterialConsumed | MaterialRestockedEvent | // etc.

// Tag to denote all sorts of material-related Events
const MaterialTag = Tag<MaterialEvent>('material')

// Tag to denote MaterialConsumed Events
const MaterialConsumedTag = Tag<MaterialConsumed>('material-consumed')

// Like the "user-fish" module, we also expose such a function – enabling other modules to "tag us."
export const getMaterialTags = (materialId: string) => MaterialTag.withId(materialId)

// We expose this function for usage by all code sites that want to log material consumption
export const emitMaterialConsumed = (
  materialInfo: MaterialInfo,
  loggedBy: User,
): Emit<MaterialConsumedEvent> => ({
  // Creating the payload is this module’s concern
  payload: makeMaterialConsumedPayload(materialInfo, loggedBy),

  // Adding the list of tags is shared concern with the user module
  // (which would like to remember material logged per-user)
  tags: getMaterialTags(materialInfo.materialId)
    .and(MaterialConsumedTag)
    .and(getUserTags(loggedBy)),
})
```

## Switch to Callback-Based baseline APIs

A short general note before we continue.

In v1, the Pond’s functions returned [RxJS](https://rxjs-dev.firebaseapp.com/) version 5 `Observable`
instances in some cases, most notably `pond.observe`.

In v2, we have switched to plain callback-style interfaces everywhere. This way, you don’t have to
figure out RxJS to get started with the Pond.
And in case you already are an ardent disciple of Reactive Programming, you are now free to plug the
Pond into the RxJS version of your choice. Please [see here](LINKPLS) for a small guide.

## Fish

A Fish is now a struct based on these fields:

- `initialState`: State of the Fish before it has seen any Events.
- `onEvent`: Function to create a new State from previous State and next Event. As with V1, this
  function must be free of side-effects; but you may now directly modify and return the old state,
  just like in
  [Array.reduce](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/reduce).
- `fishId`: A unique identifier for the Fish. This is used throughout several layers of caching, to make
  your application extra-fast. [See our docs for details.](LINKPLS)
- `where`: Which Events to pass to this Fish; like the WHERE clause in SQL.

Note that in comparison to v1, this is no longer a "factory" – you set concrete values for all
parameters.
And then you can already call `pond.observe(fish, callback)` and that’s it! Whenever our
knowledge of the Fish’s Event Log changes, we calculate a new State and pass it to your callback.

As a demonstration of this design’s flexibility, let us look at how to build a Fish that aggregates
the earliest and the latest Events for a given Tag:
```ts
type EarliestAndLatest = {
  earliest?: unknown
  latest?: unknown
}

// For non-singleton Fish, constructor functions like this are good practice
const makeEarliestAndLatestFish = (
  tag: string
): Fish<EarliestAndLatest, unknown> => {
  const initialState = {
    earliest: undefined,
    latest: undefined
  }

  const onEvent = (state: EarliestAndLatest, payload: unknown) => {
    // If `earliest` is not yet set, this is the first event we see, so we update it.
    // This works because the Pond always passes us all events in the right order!
    if (state.earliest === undefined) {
      state.earliest = payload
    }

    // Because events are passed to us in the right order,
    // every event we see is at the same time the latest event for us.
    state.latest = payload

    return state
  }

  // Listen to all Events with the given Tag.
  const where = TagQuery.requireAll(tag)

  // We uniquely identify the Fish by its 'type' and its parametrisation.
  // The version number (1) indicates the version of our program code, e.g. if we
  // changed the onEvent function, we would increase it to 2,
  // in order to invalidate the persistent cache.
  const fishId = FishId.of('earliest-latest-fish', tag, 1)

  return {
    where,
    initialState,
    onEvent,
    fishId
  }
}

// Use like this:
pond.observe(
  makeEarliestAndLatestFish('my-tag'),
  state => console.log('fish updated to new state', state)
)
```

## Command -> StateEffect

Commands are now `StateEffect`s. A StateEffect is just a function from State `S` to an array of
Event emissions `Emit<E>`.
You run one by calling `pond.run(fish, effect)`.
Functionality is the same as it was for Commands: Every Effect is guaranteed to see all Events of
earlier Effects already incorporated into the State.

State Effects can be async: You’re free to do any sort of I/O you need, before deciding which Events
to emit. For example, you might do an HTTP call based on the State, then depending on the call’s
result return an Event indicating success or failure.

Do take note, however, that as long as your State Effect is still waiting for an async operation, no
other State Effect for that specific Fish can be started, due to the serialisation guarantee. Hence
always make sure your async logic can’t stall forever.

## OnStateChange -> pond.keepRunning

Running a hook on State changes is now equivalent to just applying one and the same State Effect
again and again whenever the Fish’s State changes.
The big advantage of this: Serialisation guarantees are now also directly baked into the hook
application.
In V1 it was possible that your logic would emit the same Command multiple times, and you had to
detect this, in turn, in `onCommand`. In V2, you just don’t have to worry about this at all.

The hooks are also no longer part of the Fish itself.
You start one by calling `pond.keepRunning(fish, effect)` and get back a handle that you can use to
stop the hook at any time – observing your Fish and making it act have become two distinct things.

For example, a Fish modelling the current mission of an autonomous logistics robot can be observed
from wherever needed (including a dashboard app), but its decision logic runs only where you
explicitly start it (i.e. on the robot).

Finally, there is an optional third parameter to `keepRunning` called `autoCancel`. This can be used
to automatically uninstall your hook based on the State. For example, if your hook refers to an
individual task (modelled in a Fish) that is simply done for good at some point, your autoCancel may
read `state => state.type === 'Finished'`. 
(The hook will not resume when the condition turns false again; it will be terminated.)
