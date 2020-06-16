---
title: The Registry Fish Pattern
author: Alexander Halemba
author_title: Software Engineer at Actyx
author_url: https://github.com/Alexander89
author_image_url: /images/blog/alexander-halemba.jpg
tags: [Actyx Pond, design patterns, registry]
---

Want to keep track of and work with all fish of a specific type? Meet the RegistryFish pattern.

<!--truncate-->

## Contents

- [The problem](#the-problem)
- [The solution](#the-solution)
  - [Example](#example)
- [A general fish registry](#a-general-fish-registry)
  - [createRegistryFish](#createregistryfish)
  - [observeRegistry](#observeregistry)
- [Community package](#community-package)

## The problem

Something that has come up quite a few times is the need to track all fish of a certain kind in someplace. You may, for example, have a `MaterialRequestFish` representing a specific material request triggered by a worker or a machine. There are going to be many instances of this fish, one for each material request. Now, what if you wanted to show a list of all these open material requests somewhere?

One thing you might want to do is to implement one huge fish that tracks all instances internally, something like an `AllOpenMaterialRequestsFish`. This fish would then contain all material requests and logic for dealing with all these material requests at once.

But this will make things unnecessarily complicated. Instead of just implementing logic for one instance you will have to always deal will all the irrelevant instances. Example: even if you wanted to use just one material request to schedule a forklift or AGV, your logic will need to deal with all the irrelevant material requests as well.

There is a better way...

## The solution

A fish should always be as small as possible (think of it as a digital twin). This pattern will lead you to a data model that is **scalable**, **reuseable**, **composable**, and **maintainable**. Keeping in line with this, we split the problem into two parts:

 1. **Implement state and logic of a single entity**

- Write a fish responsible for a single instance (e.g.: `MaterialRequestFish`)

 2. **Track and access many instances of an entity**

- Write a registry fish that tracks all instances (e.g.: `MaterialRequestRegistryFish`)

This is the _Registry Fish Pattern_. It allows you to cleanly separate the concerns of the logic of an individual entity and keeping track of many instances thereof. Let's jump in with an example.

### Example

Let's take a fish representing a material request as an example. Here is how you could write this fish:

```typescript
import { FishType, FishName, OnStateChange,
         Semantics, Subscription, Pond } from '@actyx/pond'

// Very simple material request state
type State = {
  status: 'undefined' | 'created' | 'done' | 'canceled'
}
// Our fish has three kind of events create, complete, canceled
type CreateEvent = { type: 'create' }
type CompleteEvent = { type: 'complete' }
type CanceledEvent = { type: 'canceled' }
type Event = CreateEvent | CompleteEvent | CanceledEvent

// The actual fish
export const MaterialRequestFish = FishType.of<State, unknown, Event, State>({
  semantics: Semantics.of('com.example.materialRequest'),
  initialState: name => ({
    // every material request starts in the undefined state
    state: { status: 'undefined' },
    subscriptions: [
      Subscription.of(Semantics.of('com.example.materialRequest'), name)
    ],
  }),
  onEvent: (state, event) => {
    // handle the create, complete, canceled event and return the new state
    switch (event.payload.type) {
      case 'create':
        return { status: 'created' }
      case 'complete':
        return { status: 'done' }
      case 'canceled':
        return { status: 'canceled' }
      default:
        return state
    }
  },
  onStateChange: OnStateChange.publishPrivateState(),
})
```

If we now wanted to somehow deal with all material requests, we could write a registry fish as follows:

```typescript
// Map as internal fish state for a better performance and de-duplication
type FishNameMap = {[name: string]: boolean}

export const MaterialRequestRegistryFish =
  FishType.of<FishNameMap, unknown, Event, FishName[]>({

  semantics: Semantics.of('com.example.materialRequestRegistry'),
  initialState: name => ({
    // Initially the registry doesn't contain any fish names
    state: {},
    // The registry fish subscribes to the MaterialRequestFish's events
    subscriptions: [Subscription.of(MaterialRequestFish)],
  }),
  onEvent: (state, event) => {
    const { payload, source } = event
    switch (payload.type) {
      case 'create':
        // Add the fish to the registry
        state[source.name] = true
        break;
      case 'complete':
      case 'canceled':
        // Drop the source.name from the registry
        delete state[source.name]
        break;
    }
    return state
  },
  // Convert the internal map to a FishName array
  onStateChange: OnStateChange.publishState(
    intSt => Object.keys(intSt).map(FishName.of)
  ),
})
```



Here is how you could now, for example, use the `MaterialRequestRegistryFish` to show a list of all existing material request names:

```typescript
import { FishName, Pond } from '@actyx/pond'
import { MaterialRequestRegistryFish } from './materialRequestFish'

Pond.default().then(pond => {
  pond
    // Observe the registry fish
    .observe(MaterialRequestRegistryFish, FishName.of('reg'))
    // Subscribe and log to the console
    .subscribe(console.log)

}).catch(() => console.error('Is ActyxOS running?'))
```

What if you want to observe the state of the actual `MaterialRequestFish`? You can do so using [rxjs](https://www.npmjs.com/package/rxjs). Here is how:

```typescript
import { FishName, Pond } from '@actyx/pond'
import { MaterialRequestRegistryFish, MaterialRequestFish } from './materialRequestFish'
import { combineLatest } from 'rxjs/observable/combineLatest'

Pond.default().then(pond => {
  pond
    // Observe the registry fish to get the materialRequest names
    .observe(MaterialRequestRegistryFish, FishName.of('reg'))
    // Use rxjs's switchMap and switch to the MaterialRequestFish states
    .switchMap(materialRequestFishNames =>
      // Use rxjs's combineLatest to get one stream with all material requests as an array
      combineLatest(
        // For each fishName, now use the Pond to observe the actual fish
        materialRequestFishNames.map(
          materialRequestFishName => pond.observe(MaterialRequestFish, materialRequestFishName)
        )
      )
    )
    .subscribe(console.log)
}).catch(() => console.error('Is ActyxOS running?'))
```

I hope that this gives you an idea of the _Registry Fish Pattern_!

One thing that you may have noticed is that the registry is actually pretty generic. You will probably write several registry fish, which are mostly the same. In fact, this pattern just screams to be generalized. Let's see how we can do that.

## A general fish registry

So, what is needed for a generic registry?

1. We must be able to create a registry by providing event types to add fish to the registry and events to remove them.
2. We must be able to observe all the fish in the registry to get their states.

Let's jump in.

### createRegistryFish

To start from the user side, it would be nice to have a function called `createRegistryFish` which we could use like this:

```typescript
const myRegistryFish = createRegistryFish(
  // Type of the fish to be tracked by the registry
  EntityFish,
  // Events that lead to a fish being added to the registry
  ['create'],
  // Events that lead to a fish being removed from the registry
  ['done', 'delete']
)
```

To actually make this function, we must use some generics to get the type of the fish state, and we must remember how the Pond works.

First we must come up with a unique semantic for the registry fish. The semantics and the name of a fish are used to specify a fish in the Pond. That means, when we create a fish, that uses the same semantic as the referenced fish, we will get the state of that fish and not the state of our new registry fish. If we were to use a random string as semantics, we would not profit from the Pond's local snapshots.

Let's use the properties that defines our registry:

```typescript
// We assume that addEvent and removeEvent is a string or an array of strings,
// so we can use toString()
const semantics = entityFish.semantics + addEvent.toString() + removeEvent.toString()
```

Now we can implement the actual `createRegistryFish` function:

```typescript
type FishNameMap = {[name: string]: boolean}
type RegistryFishState = ReadonlyArray<FishName>

export const createRegistryFish = <E extends { type: string }>(
  entityFish: FishType<unknown, E, unknown>,
  addEvent: E['type'] | ReadonlyArray<E['type']>,
  removeEvent: E['type'] | ReadonlyArray<E['type']> = [],
) => {
  const addEvents = Array.isArray(addEvent) ? addEvent : [addEvent]
  const removeEvents = Array.isArray(removeEvent) ? removeEvent : [removeEvent]
​
  return FishType.of<FishNameMap, unknown, E, RegistryFishState>({
    semantics: Semantics.of(
      entityFish.semantics + addEvent.toString() + removeEvent.toString()
    ),
    initialState: () => ({
      state: {},
      subscriptions: [Subscription.of(entityFish)],
    }),
    onEvent: (state, event) => {
      const { payload, source } = event
      if (addEvents.includes(payload.type)) {
        // Add the fish to the registry
        state[source.name] = true
      } else if (removeEvents.includes(payload.type)) {
        // Drop the source.name from the registry
        delete state[source.name]
      }
      return state
    },
    // Convert the internal map to a FishName array
    onStateChange: OnStateChange.publishState(
      intSt => Object.keys(intSt).map(FishName.of)
    ),
    // For optimization, we can add a localSnapshot.
    localSnapshot: {
      version: 1,
      serialize: state => state,
      deserialize: state => state as FishNameMap,
    },
  })
}
```

### observeRegistry

What about observing the registry? We can create a very simple helper that returns the states of the referenced fish.

To start from the user side again, it would be nice to have a function like this:

```typescript
const entityFishStates$ = observeRegistry(
  // Registry
  RegistryFish,
  // Referenced fish
  EntityFish
  )
```

The approach we showed above is probably a bit buggy in that when the registry fish is empty or gets empty, it will not emit at all. It would be better if the `Observable` would emit an empty array. So, no fish = no entries in the array.

To fix this, we check the length of the array of known fish names in the registry, and when it is empty, a stream with an empty array is emitted. We can do this using rxjs's `Observable.of<ReadonlyArray<P>>([])`:

```typescript
export const observeRegistry = <P>(
  pond: Pond,
  registryFish: FishType<unknown, unknown, ReadonlyArray<FishName>>,
  entityFish: FishType<unknown, unknown, P>,
): Observable<ReadonlyArray<P>> =>
  pond.observe(registryFish, FishName.of('reg')).switchMap(names =>
    names.length === 0
      // return the empty array
      ? Observable.of<ReadonlyArray<P>>([])
      // return the states of the referenced fish
      : combineLatest(names.map(name => pond.observe(entityFish, name))),
  )
```

Note that we also included the Pond in this function so as to be able to actually observe the registry and its referenced fish.

### Community package

All the above functions and some more advanced features are available in the `@actyx-contrib/registry` package. Check out the [repository](https://github.com/actyx-contrib/registry) or add it to your project with `npm install @actyx-contrib/registry`.
