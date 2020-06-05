---
title: Registry fishes
author: Alexander Halemba
author_title: Software Engineer at Actyx
author_url: https://example.com
author_image_url: https://images.ctfassets.net/55iqf6xnllwu/e3hZ9KgyayE6R9G6PzibX/73d3d29a2e9bf30ed9906e0489029eff/alexander-halemba.jpg
tags: [Actyx Pond, design patterns ]
---

Want to keep track of and work with all fishes of a certain type? Meet the Registry Fish pattern.

<!--truncate-->

# The problem

One thing that has come up quite a few times is the need to track all fishes of a certain kind in some central place. You may, for example, have a `MachineFish` that represents a specific machine. There are probably going to be many instances of this fish, one for each machine (`MachineFish(machineA)`, `MachineFish(machineB)`, ...). Now what if you wanted to, for example, show a list of all these machines somewhere, or execute a command on all of these at the same time?

One thing that you might be inclined to do is use one central fish that tracks all instances internally, something like a `MachinesFish`. This fish would then contain all machines and logic for dealing with all these machines at once. This is an anti-pattern. For easier reuse and performance, you should have small fishes that represent one specific thing. There is a better way...

# The solution

Meet the _Registry Fish_:

```typescript

// Fish for which we want to create a registry
const MachineFish = FishType.of({
    ...
})

// Registry for our fish

const MachineRegistryFish = FishType.of({
    ...
})
```

Here is how you would use this `MachineFishRegistry` for showing a list of all machines:

```typescript

doSomethingSmartHere(foo)

```

Here is how you would use it to execute a command on all machines:

```typescript
doSomethingEvenSmarterHere(bar)
```

# Advanced usage

## Automating with `createRegistryFish`

Instead of creating registry fishes manuall, you can also write a `createRegistryFish` function that does so automatically. Here is an example:

```typescript
import { FishName, FishType, OnStateChange, Pond, Semantics, Subscription } from '@actyx/pond'
import { Observable } from 'rxjs'
import { combineLatest } from 'rxjs/observable/combineLatest'
​
/**
 * Create a Registry fish for a given fish definition
 *
 * ## example
 * ```typescript
 * const ExampleRegistryFish1 = createRegistryFish(ExampleFish, EventType.create)
 * const ExampleRegistryFish1 = createRegistryFish(ExampleFish, EventType.create, EventType.deleted)
 * const ExampleRegistryFish3 = createRegistryFish(ExampleFish, [EventType.create])
 * const ExampleRegistryFish4 = createRegistryFish(ExampleFish, [EventType.create], [EventType.deleted])
 * const ExampleRegistryFish5 = createRegistryFish(
 *   ExampleFish,
 *   event => {
 *     switch (event.type) {
 *       case EventType.create:
 *         return 'add'
 *       case EventType.deleted:
 *         return 'remove'
 *       default:
 *         return 'ignore';
 *     }
 *   }
 * )
 * ```
 *
 * @param entityFish Fish to create the Registry for
 * @param addEventOrEventHandler EventType or Array of EventTypes to add the source.name to the Registry or an eventHandler for mor complex Registry use-cases
 * @param removeEvent EventType or Array of EventTypes, when the source.name should be removed from the Registry
 */
export const createRegistryFish = <E extends { type: string }>(
  entityFish: FishType<unknown, E, unknown>,
  addEventOrEventHandler: E['type'] | ReadonlyArray<E['type']> | RegistryOnEvent<E>,
  removeEvent: E['type'] | ReadonlyArray<E['type']> = [],
) => {
  return FishType.of<RegistryFishState, unknown, E, RegistryFishState>({
    semantics: Semantics.of(entityFish.semantics + 'AutoRegistry'),
    initialState: () => ({
      state: [],
      subscriptions: [Subscription.of(entityFish)],
    }),
    onEvent: (state, event) => {
      const { payload, source } = event
      if (typeof addEventOrEventHandler === 'function') {
        switch (addEventOrEventHandler(payload)) {
          case 'add':
            return state.includes(source.name) ? state : [...state, source.name]
          case 'remove':
            return state.filter(name => name !== source.name)
          case 'ignore':
            return state
        }
      } else {
        const addEvents: ReadonlyArray<E['type']> = Array.isArray(addEventOrEventHandler)
          ? addEventOrEventHandler
          : [addEventOrEventHandler]
        const removeEvents: ReadonlyArray<E['type']> = Array.isArray(removeEvent)
          ? removeEvent
          : [removeEvent]
​
        if (addEvents.includes(payload.type)) {
          return state.includes(source.name) ? state : [...state, source.name]
        } else if (removeEvents.includes(payload.type)) {
          return state.filter(name => name !== source.name)
        } else {
          return state
        }
      }
    },
    onStateChange: OnStateChange.publishPrivateState(),
    localSnapshot: {
      version: 1,
      serialize: state => state,
      deserialize: state => state as ReadonlyArray<FishName>,
    },
  })
}
```

## Observe registry fishes

Here is a helper function you could use to observe registry fishes:

```typescript
/**
 * observeRegistry can be used to map the state of an registryFish to the entity fish
 *
 * @see observeRegistryMap map the registry fish state to a FishName[]
 *
 * @param pond pond instance or pond.observe function
 * @param registryFish Registry fish which state is a FishName[]
 * @param entityFish entity fish to observe the state
 */
export const observeRegistry = <P>(
  pond: PondObserve,
  registryFish: FishType<unknown, unknown, ReadonlyArray<FishName>>,
  entityFish: FishType<unknown, unknown, P>,
): Observable<ReadonlyArray<P>> =>
  obs(pond)(registryFish, FishName.of('reg')).switchMap(names =>
    names.length === 0
      ? Observable.never<ReadonlyArray<P>>().startWith([])
      : combineLatest(names.map(name => obs(pond)(entityFish, name))),
  )
```

## Use the `registry-helper.ts`

All the function above can be downloaded as a single file you can add to your project [here](http://example.com).