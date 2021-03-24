---
title: "Introducing observeAll() and observeOne() for Actyx Pond"
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [Actyx Pond, TS, TypeScript, observeAll, observeOne]
---

With the release of Pond 2.3.0 we are shipping two new major functions: `Pond.observeAll()` and `Pond.observeOne()` – they will make development a lot easier in some cases.

Sneak peak:  
`pond.observeAll(taskCreatedTag, makeTaskFish, {}, x => console.log('all tasks:', x))`

In this blog post we give an overview of the new functions and explain the motivation behind adding them.
<!-- truncate -->

## Observe All Existing Things

`Pond.observeAll()` takes three arguments and a callback.

- `seedEvents: Where<F>` – A selector for events of type `F` – **F** is for **F**irst Event
- `makeFish: (f: F) => Fish<S, any>` – A factory function that creates a `Fish<S, any>` from an event of type `F`
- `opts: ObserveAllOpts` - An object containing optional arguments
- `callback: (states: S[]) => void` – The callback receives an array of all known states!

Hopefully the way this API works is almost self-explanatory.
For each selected "seed event" `F`, a `Fish` is spawned using the supplied factory function.
Whenever the set of Fish changes (newfound `F`), or the state of a Fish inside the set changes (changed `S`), the callback is invoked with the updated array of all states.

One question remains: How to keep the list of states from growing ever longer?  
There is no clear cut answer. Depending on the scenario, different conditions make a Fish irrelevant: A task may be fulfilled, or a task may become too old.  
This is where the `ObserveAllOpts` argument comes in. In the future it will enable a variety of pruning options.
For now we start with the most simple one: `expireAfterSeed`, meaning Fish are dropped when their initial event `F` has become too old.
Using this setting, we may for example observe all tasks created in the last 24 hours:

```ts
// Event definitions:
type TaskStarted  = // ...
type TaskFinished = // ...
type TaskCreated  = // ...

const taskChangedTag = Tag<TaskStarted | TaskFinished>('task-changed')
const taskCreatedTag = Tag<TaskCreated>('task-created')

type TaskState = {
  id: string
  name: string
  description: string
  status: 'open' | 'in-progress' | 'finished'
}

// Fish is built from a TaskCreated event rather than from thin air!
const makeTaskFish = (taskCreated: TaskCreated): Fish<TaskState, TaskChanged> => ({

  // It’s ok if we don’t subscribe to TaskCreated here
  where: taskChangedTag.withId(taskCreated.taskId),

  // Note how we can fill in all mandatory fields already
  initialState: {
    id: taskCreated.taskId,
    name: taskCreated.name,
    description: taskCreated.description,
    status: 'open'
  },

  onEvent,
  fishId: FishId.of('task', taskCreated.taskId, 1)
})

pond.observeAll(
  taskCreatedTag,
  makeTaskFish,
  { expireAfterSeed: Milliseconds.fromDays(1) },
  (states: TaskState[]) => console.log('all tasks of the last 24 hours:', states)
)
```

If you want to filter out Fish depending on their state, you can just do so manually for the time being:

```ts
const callback = (states: TaskState[]) =>
  console.log('all open tasks:', states.filter(state => state.status !== 'finished'))
```

For the future we are envisioning an option allowing to specify  
`{ expireWhen: Tag('task-finished') }`  
which would retire Fish from the set as soon as they have consumed a `TaskFinished` event.
The advantage over filtering manually would be that the Fish can actually be stopped internally, and will no longer take up resources.

[Read our detailed documentation on observeAll.](/docs/pond/in-depth/observe-all)

## Observe One Specific Thing

If you’re looking to observe a specific task, you can re-use `makeTaskFish` and call `observeOne` instead of `observeAll`:  

```ts
const taskCreatedTag = Tag<TaskCreated>('task-created')

pond.observeOne(
  // Find a specific seed event F:
  taskCreatedTag.withId('specific-task-id'),
  makeTaskFish,
  // Will only be called once the seed event has been found:
  (state: TaskState) => console.log(state)
)
```

The "One" in `observeOne` means that if there are multiple events matching the selector for `F`, then one of them is chosen, according to no specific logic.
So there should either be just one event matching the selector, or it should not matter which one is used.  
In the example, we would assume that for this specific ID there is just a single `TaskCreated` event.

[Read our detailed documentation on observeOne.](/docs/pond/in-depth/observe-one)

## What We’re Aiming to Improve

Motivation behind these functions comes from two long-standing quirks of the Pond.

The first one’s a common question by first-time Pond developers:  
"I have defined my Fish type, now how can I get all existing Fish of this type?" (Pond V1)  
Our answer used to be: [Write a Registry Fish.](/blog/2020/06/16/registry-fishes)
But actually that can be rather cumbersome.
`observeAll` is a new way to enumerate Fish and in most situations it will be simpler than writing a custom Registry Fish.
It also has better performance: Where the Registry Fish will time travel like a regular Fish,
`observeAll` knows that the order of event application for "registry" logic is most of the time irrelevant.

The second quirk we’re getting rid of is the problem of the empty initial state.
Often you would be modelling entities with clearly mandatory fields, e.g. "every task has a description."
But when you passed your `TaskFish` to `observe`, your code couldn’t know the description.
Still it had to give an `initialState` for the `Fish`.
There were elegant workarounds for this issue, but in the end, all of them added boilerplate.
`observeOne` and `observeAll` both build the `initialState` from the seed event;
so when your `TaskCreated` event contains the description, you can already put it into the `initialState`.

## Future Work

This release is but one step towards making expression of distributed programs the easiest it can be.
The Pond is a very powerful tool; a Fish can express everything, but at the price of some complexity.
One thing we’re researching is: Could there be an alternative library, that’s a bit less powerful, but also much simpler?

Stay tuned!
