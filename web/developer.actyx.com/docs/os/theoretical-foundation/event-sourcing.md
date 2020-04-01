---
title: Event Sourcing
---

What is event sourcing and why is it relevant in decentralized computing?

## Contents

- [Definition](#definition)
- [Benefits](#benfits)
- [Challenges](#challenges)
- [Relevance to ActyxOS](#relevance-to-actyxos)
- [Example](#example)
- [Learn more](#learn-more)

## Definition

_Event sourcing_ is an architectural pattern in which state is not stored directly, but rather computed _as-needed_ from events stored in an event log.

## Benefits

### Auditability

By using an _append-only event log_, you have an easily auditable, and complete history of what has happened in your system. This means you can always understand exactly what happened and when.

### Orthogonality

By separating the fundamental store from the computed view (state), you separate the _what has happened_ concern from the _how to look at it_ concern.

### Extensibility

Because you always know what has happened in the past, you can&mdash;from the future&mdash;change how to interpret the past. This is something you could never do with a traditional state-based system.

## Challenges

### Performance

As the size of the event store increases, the amount of time it takes to compute a state may increase if you don't remember the previous state you computed. _Snapshots_ (see the Actyx Pond [documentation](../../pond/guides/snapshots.md)) can help mitigate this.

### Reasoning

Because of the separation of events and state, reasoning about the system can be more difficult unless you compute the current state.

### Migrations

Event schema migrations can pose serious challenges, especially if you want to migrate without deleting past events&mdash;which you can't do if they affect your state.

_Check out [how the Actyx Pond deals with this](../../pond/guides/types.md)_.

## Relevance to ActyxOS

ActyxOS provides you with the basic tools you need to build a decentralized event sourcing system. The Event Service's [persistent event streams](../guides/event-streams.md) allow you to model a distributed _append-only_ log&mdash;indeed, that is what they were designed for. The [WebView Runtime of ActyxOS on Android](../advanced-guides/actyxos-on-android) and [Docker Runtime of ActyxOS on Docker](../advanced-guides/actyxos-on-docker.md) allow you to run apps that consume these event streams, thus allowing you to compute state.

> Actyx Pond
>
> Check out the [Actyx Pond](../../pond/introduction)&mdash;an auxiliary product to ActyxOS&mdash;which provides you with an always-available, partition-tolerant, event-sourcing system out of the box. It also tries to mitigate some of the key associated challenges.

## Example

Consider, for instance, a truck being loaded with shipping boxes. At any one point in time the truck will have a loading state. If we were to track this state programmatically we might write an object as follows:

```js
var loadingState = {
    totalLoadedWeight: 753,
    loadedPackages: [
        {
            id: "5b4f8ffd-4531-4b05-9268-a56b78a32cd2",
            destination: "John Doe, 4540 1st Street, 10001 NY, USA",
            weight: 3.2,
        }
        // more packages
        // ...
    ]
}
```

Now, whenever a package is loaded or unloaded from the truck by a worker (or robot), we might adjust the state as follows:

```js
var loadedPackaged = {
    id: "dd274baf-09f4-4024-8081-bf74bb3f1715",
    destination: "Jane Doe, 1001 Main Street, 34333 MI, USA",
    weight: 12,
}

loadingState = {
    totalLoadedWeight: loadingState.totalLoadedWeight.concat(loadedPackage.weight),
    loadedPackages: loadingState.loadedPackaged.concat(loadedPackag)
}
```

With this approach, we are continuously keeping track of the state and updating it as things change.

> Note
>
> This is how most software systems are built, with the state being held in large databases and [CRUD operations](https://en.wikipedia.org/wiki/Create,_read,_update_and_delete) leading to state changes.

Using an **event sourcing architecture** we would take a different approach. Let's have a look.

Firstly we would define two types of events that may happen in our system&mdash;and that may affect our state:

```js
// PackageLoaded event
const PackageLoaded = {
    type: "PackageLoaded"
    package: {
        // package details
    }
}

// PackageUnloaded event
const PackageUnloaded = {
    type: "PackageUnloaded"
    package: {
        // package details
    }
}
```

We would then build an event store&mdash;more precisely an _append-only event log_&mdash;that we append new events to whenever they happen:

```js
var eventLog = [] // initially nothing has happened

// First event happens
eventLog.concat(firstEvent);

// Second event happens
eventLog.concat(secondEvent);

// Etc...
```

> Append-only!
>
> Unless you have very good reasons for doing so, you should never remove an event from an append-only event log. If you want to undo something, in most cases, the right approach is to define a compensating event that undoes what a previous event may have done.

What if we now want to find out the current loading state of our truck? We need two things for this work. Firstly, an initial state, i.e. what was the loading state when the truck came off the production line. Secondly, a function that computes a state from events. Let's build both:

```js
// The initial state
const initialState = {
    totalLoadedWeight: 0,
    loadedPackages: []
}

// The function that computes the current state from the initial state and a list
// of events, i.e. the event log
function computeState(events) {
    var state = initialState;
    events.map(event => {
        if (event.type === "PackageLoaded") {
            // update the state
            state = {
                totalLoadedWeight: state.totalLoadedWeight + event.package.weight,
                loadedPackages: state.loadedPackages.concat(event.package),
            }
        } else if (event.type === "PackageUnloaded") {
            // remove the package from the list of packages
            var loadedPackages = state.loadedPackaged;
            var index = loadedPackages.indexOf(event.package);
            if (index !== -1) loadedPackages.splice(index, 1);
            // update the state
            state = {
                totalLoadedWeight: state.totalLoadedWeight - event.package.weight,
                loadedPackages: loadedPackages
            }
        }
    })
}
```

Now, if we want to know the current loading state of the truck we must simply call the `computeState` function and pass it to our current event log.

> More idiomatic implementation
>
> In reality, you would not implement your system this way. You would, rather, define an `onEvent` function that takes a current state and a _single_ event and computes a new state. Then you would repeatedly call that function for each event.

## Learn more
- Martin Flowler's [introducton to event sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)
- [Event Sourcing Pattern (Microsoft)](https://docs.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
