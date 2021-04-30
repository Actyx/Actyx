---
title: The Registry Fish Pattern (Pond V2)
author: Alexander Halemba
author_title: Developer Advocate at Actyx
author_url: https://github.com/Alexander89
author_image_url: /images/blog/alexander-halemba.jpg
tags: [Actyx Pond, design patterns, registry]
---

Want to keep track of and work with all fish of a specific type? Meet the RegistryFish pattern. (_Updated version to Pond V2_)

<!--truncate-->

## The problem

Something that has come up quite a few times is the need to track all fish of a certain kind. You may, for example, have a `MaterialRequestFish` representing a specific material request triggered by a worker or a machine. There are going to be many instances of this fish, one for each material request. Now, what if you wanted to show a list of all these available material requests somewhere?

One thing you might want to do is to implement one huge fish that tracks all instances internally, something like an `AllOpenMaterialRequestsFish`. This fish would then contain all material requests and logic for dealing with all these material requests at once.

But this will make things unnecessarily complicated. Instead of just implementing logic for one instance, you will always have to deal with all the irrelevant instances. For example: even if you wanted to use just one material request to schedule a forklift or AGV, your logic would need to deal with all the irrelevant material requests as well.

There is a better way...

## The solution

A fish should always be as small as possible, it should model only the one object or workflow it corresponds to. This pattern will lead you to a data model that is **scalable**, **reusable**, **composable**, and **maintainable**. Keeping in line with this, we split the problem into two parts:

<!-- markdownlint-disable MD029 -->
1. **Implement state and logic of a single entity**

- Write a fish responsible for a single instance (e.g.: `MaterialRequestFish.of(id)`)

2. **Track and access many instances of an entity**

- Write a registry fish that tracks the names or the id of all instances (e.g.: `MaterialRequestFish.registry`)
<!-- markdownlint-enable MD029 -->

This is the _Registry Fish Pattern_. It allows you to cleanly separate the concerns of the logic of an individual entity and keeping track of many instances thereof. Let's jump in with an example.

### Example

Let's take a fish representing a material request as an example. Here is how you could write this fish:

```typescript
import { Fish, FishId, Pond } from '@actyx/pond'

// Very simple material request state
type State = {
  status: 'undefined' | 'created' | 'done' | 'canceled',
  id: string
}
// Our fish has three kind of events create, complete, canceled
type Event = {
  eventType: 'MaterialRequestCreated' | 'MaterialRequestCompleted' | 'MaterialRequestCanceled',
  id: string
}
const materialRequestTag = Tag<Event>('com.example.materialRequest')

// The actual fish
export const MaterialRequestFish = {
  of: (id: string): Fish<State, Event> => ({
    fishId: FishId.of('com.example.materialRequest', id, 0),
    initialState: {
      status: 'undefined',
      id
    },
    // subscribe to the events of tagged with the materialRequestTag + id
    where: materialRequestTag.withId(id),
    // handle the create, complete, canceled event and return the new state
    onEvent: (state, event) => {
      switch (event.eventType) {
        case 'MaterialRequestCreated':
          return { status: 'created', id }
        case 'MaterialRequestCompleted':
          return { status: 'done', id }
        case 'MaterialRequestCanceled':
          return { status: 'canceled', id }
        default:
          return state
      }
    },
  }),
}
```

If we now wanted to somehow deal with all material requests, we could write a registry fish as follows:

```typescript
// A map as state for better performance and de-duplication
type IdMap = Record<string, boolean>

export const MaterialRequestFish = {
  registry: ({
    fishId: FishId.of('com.example.materialRequest.registry', 'registry', 0),
    // start with an empty map
    initialState: {},
    // in this example, we require all events in this tag.
    // You have to select/tag them more carefully in your advanced event-streams
    // check out the docs about tags: https://developer.actyx.com/docs/how-to/actyx-pond/in-depth/tag-type-checking
    where: materialRequest,
    // handle the create, complete, canceled event and return the new state
    onEvent: (state, event) => {
      switch (event.eventType) {
        case 'MaterialRequestCreated':
          // Add the id to the registry
          state[event.id] = true
          break;
        case 'MaterialRequestCompleted':
        case 'MaterialRequestCanceled':
          // Drop the event.id from the registry
          delete state[event.id]
          break;
      }
      return state
    },
  } as Fish<IdMap, Event>),
})
```

Here is how you could now, for example, use the `MaterialRequestFish.registry` to show a list of all existing material request names:

```typescript
import { FishName, Pond } from '@actyx/pond'
import { MaterialRequestFish } from './materialRequestFish'

Pond.default().then(pond => {
  // Observe the registry fish and log the state to the console
  pond.observe(MaterialRequestFish.registry, state => console.log(state))
}).catch(() => console.error('Is ActyxOS running?'))
```

What if you want to observe the state of the actual `MaterialRequestFish`? You can do so using the [RxPond](https://www.npmjs.com/package/@actyx-contrib/rx-pond). Here is how:

```typescript
import { Pond } from '@actyx/pond'
import { RxPond } from '@actyx-contrib/rx-pond'
import { MaterialRequestFish } from './materialRequestFish'
import { combineLatest } from 'rxjs'
import { switchMap } from 'rxjs/operators'

// assuming that we already have a pond instance in your application
Pond.default().then(pond => {
  RxPond.from(pond)
    // Observe the registry fish to get the materialRequest ids
    .observe(registryFish)
    // we will map the list of Ids to the entities with the RxJS pipeline
    // find more information about RxJS here: https://www.npmjs.com/package/rxjs
    .pipe(
      // switch over to the entity fish
      switchMap((idMap) =>
        // Use RxJS's combineLatest to get one stream with all material requests as an array
        combineLatest(
          // map the id of the state to an EntityFish
          Object.keys(idMap).map(id =>
            // observe a fish of each entry in the ids array
            rxPond.observe(makeEntityFish(id)),
          ),
        ),
      ),
    )
    // subscribe to the stream to get all entity states
    .subscribe(allEntityStates = console.log(allEntityStates))
}).catch(() => console.error('Is ActyxOS running?'))
```

Hopefully, this snippet gives you an idea of the _Registry Fish Pattern_!

One thing that you may have noticed is that the registry is pretty generic and could be used all over different projects. Let's see how we can pack that into an npm package.

## Pack it. Ship it!

It would be convenient to have a module to observe all entities fish in the registry.

By the way, this pattern is not only useful for a registry and its entities. We can use it to resolve references in one fish and wake up other fish according to a given field. E.g.: Forklift -> current job / material request -> production order.

### observeRegistry

We can create a very simple helper that returns the states of the referenced fish.

To start from the user perspective, it would be nice to have a function like this:

```typescript
const entityFishStates$ = observeRegistry(
  // Registry
  RegistryFish,
  // Referenced fish
  EntityFish
)
```

The approach we showed above is probably a bit buggy when the registry fish is empty or gets empty; it will not emit at all. It would be better if the Observable would emit an empty array. So, no fish = no entries in the array.

To fix this, we check the length of the array of known fish names in the registry, and when it is empty, a stream with an empty array is emitted. We can do this using RxJS's `Observable.of<ReadonlyArray<P>>([])`.

Additionally, we could improve the performance a lot if we rebuild the pipeline only if the state of the registryFish changed. RxJS's `distinctUntilChanged(deepEqual)` will do this for us out of the box.

```typescript
export const observeRegistry$ = <RegS, Prop, State>(
  rxPond: RxPond,
  registryFish: Fish<RegS, unknown>,
  mapToProperty: (regState: RegS) => ReadonlyArray<Prop | undefined>,
  makeEntityFish: (p: Prop) => Fish<State, unknown>,
): Observable<State[]> =>
  rxPond.observe(registryFish).pipe(
    // just emit when the registry changed
    distinctUntilChanged(deepEqual),
    // convert teh map to an array of properties
    map(mapToProperty),
    // filter out unset properties to protect fish from bad properties
    map((props): Prop[] => props.filter((p): p is Prop => p !== undefined)),
    // switch over to the entity fish
    switchMap((ids) => ids.length === 0
      // return empty array when registry is empty
      ?  of([])
      // Use RxJS's combineLatest to get one stream with all material requests as an array
      : combineLatest(
          // map the id of the array to an EntityFish
          ids.map(id =>
            // observe a fish of each entry in the ids array
            rxPond.observe(makeEntityFish(id)),
          ),
        ),
    ),
  )
```

Note that we also included `pond` as a parameter to observe the registry and its referenced fish. Finally, the `mapToProperty` function will give you the freedom to may any state to an array of properties.

Here is an example:

```typescript
import { RxPond } from '@actyx-contrib/rx-pond'
import { observeRegistry$ } from '@actyx-contrib/registry'
import { MaterialRequestRegistryFish } from './materialRequestFish'
// import { ForkliftFish } from './forkliftFish'

RxPond.default().then(pond => {
  observeRegistry$(
    pond,
    MaterialRequestFish.registry,
    Object.keys,
    MaterialRequestFish.of,
  ).subscribe(entityStates => console.log(entityStates))

  /*
  observeRegistry$(
    pond,
    ForkliftFish.of('RoadRunner 1'),
    state => [state.currentMaterialRequest],
    MaterialRequestFish.of,
  ).subscribe(([currentMaterialRequest]) => console.log(currentMaterialRequest))
  */
})
```

### non-RxJS version

Suppose you are not familiar with RxJS or focus on other things. I add a wrapper around the `observeRegistry$` function in the node package. It is named `observeRegistry` without the `$`, and it has an additional parameter for any stateChanged callback.

Here, additional an example:

```typescript
import { Pond } from '@actyx/pond'
import { observeRegistry } from '@actyx-contrib/registry'
import { MaterialRequestRegistryFish } from './materialRequestFish'

Pond.default().then(pond => {
  observeRegistry(
    pond,
    MaterialRequestFish.registry,
    Object.keys,
    MaterialRequestFish.of,
    entityStates => console.log(entityStates)
  )
})
```

## Community package

All the above functions, including the non-RxJS version, are available in the `@actyx-contrib/registry` package. Check out the [repository](https://github.com/actyx-contrib/registry) or add it to your project with `npm install @actyx-contrib/registry`.
