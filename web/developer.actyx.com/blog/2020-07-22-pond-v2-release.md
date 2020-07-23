---
title: Introducing Pond Version 2
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [Actyx Pond]
---

We are happy to announce the release of the Actyx Pond Version 2. [Download from npm](LINKPLS)

Read on for a brief overview of the changes, which we have developed with the goal of reducing
boilerplate and allowing more flexibility and elegance in your application architecture.

<!-- truncate -->

## Tags

The biggest change we’re rolling out to the Pond and the ActyxOS EventService in general with
this version is that _events_ are now indexed based on _tags_ assigned by your application. There can be
any number of tags given for an event. That means an event no longer belongs to one single _stream_
identified by _semantics_ and _fishName_, but instead can belong to many streams – each identified by just a
string, as tags are nothing but strings.

To retrieve events based on their tags, you can then employ logic like:
- Events with tag 'foo'
- Events with tag 'foo' or Tag 'bar' (or both)
- Events with both tags 'foo' and 'bar'

Additional tags are always O.K., so if an event has tags `['foo', 'bar', 'baz']` it will also
match, in all three cases.

For the Pond, we are shipping multiple nice mechanisms for expressing your tag-based queries.

A very quick demonstration ([detailed docs](LINKPLS)):

```typescript
// Select events with either both tag0 and tag1, or just tag2.
// (Events with all three tags also match.)
const where = TagQuery.matchAnyOf(
  'tag2',
  TagQuery.requireAll('tag0', 'tag1'),
)

// Alternatively, declare your tags in association with event types:
const Tag0 = Tag<Type0>('tag0')
const Tag1 = Tag<Type1>('tag1')
const Tag2 = Tag<Type2>('tag2')

// And then use fluent type-checked query building
const where = tag2.or(tag0.and(tag1))

```

Also be sure to check out [our guide on how to design your application architecture based on tags](LINKPLS).

## Direct Event Emission

In version 1 of the Actyx Pond, all events had to be emitted by _Fish_, from a received _command_.
Now, events can be emitted freely without any Fish at hand.
```typescript
pond.emit(['myFirstTag', 'mySecondTag'], myEventPayload)
```

It is still recommended that you organize ownership of events (by type) into modules, for example:

```typescript
type MaterialConsumed = // The type you have designed

// Sum of all types related to material
type MaterialEvent = MaterialConsumed | MaterialRestockedEvent | // etc.

// Tag to denote any sort of material-related event
const MaterialTag = Tag<MaterialEvent>('material')

// Tag to denote MaterialConsumed events
const MaterialConsumedTag = Tag<MaterialConsumed>('material-consumed')

// A module modelling users, exposing a function to ahold of the tags
// that should be attached to all user-related events.
import { getUserTags } from './user-fish'

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

## Switch to Callback-Style APIs over Observables and Promises

A short general note before we continue.

In v1, the Pond’s functions returned [RxJS](https://rxjs-dev.firebaseapp.com/) version 5 `Observable`
instances in some cases, most notably `pond.observe`.

In v2, we have switched to plain callback-style interfaces everywhere. This way, you don’t have to
figure out RxJS to get started with the Pond.
And in case you already are an ardent disciple of Reactive Programming, you are now free to plug the
Pond into the RxJS version of your choice. Please [see here](LINKPLS) for a small guide.

## Fish

A Fish is now a struct based on these fields:

- `initialState`: State of the Fish before it has seen any events.
- `onEvent`: Function to create a new state from previous state and next event. As with v1, this
  function must be free of side-effects; but you may now directly modify and return the old state,
  just like in
  [Array.reduce](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/reduce).
- `fishId`: A unique identifier for the Fish. This is used throughout several layers of caching, to make
  your application extra-fast. [See our docs for details.](LINKPLS)
- `where`: Which events to pass to this Fish; like the WHERE clause in SQL.

Note that in comparison to v1, this is no longer a "factory" – you set concrete values for all
parameters.
And then you can already call `pond.observe(fish, callback)` and that’s it! Whenever our
knowledge of the Fish’s event log changes, we calculate a new state and pass it to your callback.

As a demonstration of the design’s flexibility, let us look at how to build a Fish that aggregates
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
  const fishId = FishId.of('earliest-latest-fish', tag, /* program code version: */ 1)

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

## Emitting Events that depend on State

We have revamped the whole command system in order to make it much more straight-forward to
use. As mentioned above, you can now emit events directly, so there is no longer a need for commands
in the general case. You will only have to employ them in those cases where you need the local
serialisation guarantee:

<!-- fancy formatting maybe -->
_Emit some events depending on locally known state of a Fish. Then do the same thing again, but
guaranteed to see all formerly emitted events already applied to the state._

This is very useful in all cases where you have one node executing tasks that other nodes ask
for. Take for example an autonomous logistics robot that is bringing material to production
machines. On the production machines, there is an ActyxOS app emitting events that announce the
task; on the robot, there is an app which looks at the open tasks and tries to fulfill them in the
real world, by driving the robot.

Whenever a task is done, another event needs to be published, indicating the task being done. So
what the robot does is:

- Look at a Fish that collects open tasks into a list
- Execute a task in the real world
- Publish "TaskDone" event
- Loop back to the beginning

But how can the robot be sure that the TaskDone event has already arrived at the Fish keeping the
list of open tasks? In case the event hadn’t arrived yet, the robot might start on the same task a
second time! This is where `pond.keepRunning` comes in.

```typescript
pond.keepRunning(fish, async (state, enqueue) => {
  if (fish.tasks.length === 0) {
    return
  }

  const nextTask = fish.tasks[0]

  // Deliver material, for example
  await executeInRealWorld(task)

  const taskDoneEvent = makeTaskDoneEvent(task)
  const tags = getTags(taskDoneEvent)

  // Queue the TaskDone event for emission;
  // the next invocation of this function will see it already applied to `state`.
  enqueue(taskDoneEvent, tags)
})

```

The Pond will invoke the function you pass as argument whenever the Fish’s state changes. You can
call `enqueue` any number of times to enqueue events for emission: The next time your function is
invoked, all previously enqueued events will be part of the state already.

If you don’t want your logic to keep running forever, you can:

- Use `pond.run` to execute your logic just once, but serialised in regards to all previous local
  invocations of `pond.run`, and active `pond.keepRunning` effects.
- Or set the optional third argument to `pond.keepRunning`, called `autoCancel`. It can be used to cancel
  your logic for good, once a certain state of the Fish is reached. For example, if you’re modelling
  tasks as individual Fish requiring a number of steps, you may want to stop once the final state is
  reached: `autoCancel = (state) => state.type === 'Finished'`.
