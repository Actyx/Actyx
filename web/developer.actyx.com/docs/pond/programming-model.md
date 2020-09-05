---
title: Programming Model
---

The **Actyx Pond** is an opinionated TypeScript framework for writing distributed apps preferring availability over consistency.

At the core of the Actyx Pond lies an innovative programming model: business logic is written such that it reacts to the reception of new facts—called events—without needing to care where these facts are generated or how they are transported to the piece of logic that needs them. The logic entities swim in the datalake and breathe events according to their needs.  Hence, we call them _fishes_.

## Fish Identities

Each fish has a unique identity in the whole swarm (linguistic note: fish form schools, not swarms; the use of the term “swarm” stems from peer-to-peer systems like the underlying IPFS technology that apparently views individual devices like flying insects). The full identifier of a fish has three parts:

1. the _fish type_ specifies the purpose of the fish, e.g. thermometer readings
2. the _name_ distinguishes this fish from other fish of the same semantics, e.g. the location of the physical thermometer
3. the _version_ of the fish (this needs to be incremented whenever the fish's business logic changes)

![devices](/images/pond/fishes-on-devices.png)

Each event a fish emits, can be provided with an arbitrary amount of tags, e.g. with a semantic identification of the event (`temperatureReading`) and a description about its origin (`thermometerLocation:hotEnd`). Note that event emission does not require a fish, but can also be done without one.

It is important to note that the “same” fish—identified by fish type & name—can run on different devices, each having its own identity and producing its own distinct emitted events.

## How fishes communicate

Fishes communicate with other fishes by way of event subscriptions, i.e. a fish can declare interest in the events emitted by another fish.

This is done by supplying a query describing the tags the desired events should have and forming a _subscription_ from that, it's also possible to only subscribe to local events ignoring events originating from other nodes; the usage of this concept will be discussed in the next section.

## Fishes reading events

Fishes do not act on their own accord, they only react when something happens: a new event becomes available.
The primary function of a fish is to accumulate local knowledge — state — from the incoming events it has subscribed to.
One **very important note** is that events become available at each edge device individually, whenever devices can talk to each other and exchange the latest information.
Therefore, if you run the same fish logic with the same subscriptions on different devices, they will receive events at different times or in different order; consequently, the current state computed at each device may temporarily be different.
But Actyx Pond will ensure that eventually — when the device has had the chance to catch up with the latest information — all fishes will have seen the same events and will have computed the same state.

![reading](/images/pond/fish-reading-events.png)

The illustration shows that besides the events there is one more input, namely the initial state from which the fish will start before it has seen any events.
The `onEvent` handler is a function that takes the current state and the next event and computes the next state from that.
State are made observable outside the fish by APIs made available through the `Pond` object.

One very important consequence of this computational model is that the computed state can be recreated whenever needed, by replaying the input events.
The state is not the most important part, it is not persisted.
In contrast to a database system that only stores the current state, a fish can be fixed retroactively by removing a bug in the business logic and reprocessing all events again.

:::info Remember
Fishes compute their current state by deterministically applying the subscribed event streams.
:::

## Writing events

Events emission is not coupled to the usage of fishes, but is conceptually treated together. Event creation can be triggered from anywhere in the application, from an external system (e.g. via an HTTP call), or from a human operator of the app.

![emitting](/images/pond/fish-emitting-events.png)

Usually, the current state as derived from all locally known events of a fish is interpreted in order to emit an event.
This is called a state effect. A `StateEffect` is a function, accepting the derived state of a fish, and returning a list of events (consisting of tags and a payload) to be emitted.  The list of events may be empty, or it may record the fact that an invalid condition was encountered, whichever is required by the business requirements.

It is important to note that the fish will not change its state alone by emitting a state effect; it needs to emit an event, which is written to the event stream and then passed into the `onEvent` handler as usual (if the emitted tags match the fish's subscription), where the state can be changed.  This is done in order to make all state changes
reliably repeatable after a crash or restart of the app — only events are persistent, state effects are not recorded and thus also not replayed.

:::info Remember
Fishes record facts (including environmental observation as well as decisions) by emitting events; fact generation is not deterministic and depends on the currently known and possibly incomplete state.
:::

## Availability vs. Consistency

As we have discussed above, the current state of a fish on an edge device may still be missing information that is available elsewhere in the swarm.
The most distinctive characteristic of the Actyx Pond framework is that it allows this fish to still make progress — process commands and emit events — even though this might lead to inconsistencies for a human looking at the whole system.
This trade-off of favoring availability over consistency is a fundamental one, it is impossible to have both, as is also explained at [ActyxOS and CAP](../os/theoretical-foundation/actyxos-and-cap).

We made this choice because a mission-critical environment like a factory shop-floor is built around this same choice already:
groups of persons are working with machines and material to deliver the required products, independent of other processes ongoing on the shop-floor around them.
In such a collaborative setting it is more important to make progress individually than to ensure that every stakeholder on the factory shop-floor has a consistent view on the overall state.
