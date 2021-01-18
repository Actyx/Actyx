---
title: Programming Model
hide_table_of_contents: true
---

The **Actyx Pond** is an opinionated TypeScript framework for writing distributed apps preferring availability over consistency.

At the core of the Actyx Pond lies an innovative programming model: business logic is
written such that it reacts to the reception of new facts—called events—without needing to care where these facts are generated or how they are transported to the piece of logic that needs them. The logic entities swim in the datalake and breathe events according to their needs.  Hence, we call
them _fishes_.

## Fish Identities

Each fish has a unique identity in the whole swarm (linguistic note: fish form schools, not swarms; the
use of the term “swarm” stems from peer-to-peer systems like the underlying IPFS technology that
apparently views individual devices like flying insects). The full identifier of a fish has three parts:

1. the _semantics_ specifies the meaning of events produced by the fish, e.g. thermometer readings

2. the _name_ distinguishes this fish from other fish of the same semantics, e.g. the location of
   the physical thermometer

3. the _source_ denotes the device on which this instance of the fish is currently being run; the
   source is assigned automatically by the ActyxOS runtime

![devices](/images/pond/v1-fish-on-devices.svg)

The fish identifier is at the same time the name of the event stream emitted by this fish. It is important
to note that the “same” fish—identified by semantics & name—can run on different devices, each
having its own identity and producing its own distinct event stream. This is the reason for
including the source identifier in the full fish identity.

## How fishes communicate

Fishes communicate with other fishes by way of event subscriptions, i.e. a fish can declare interest
in the events emitted by another fish. This is done by supplying the name of the target fish as a
triplet of semantics, name, and source and forming a _subscription_ from that; the usage of this
concept will be discussed in the next section.

![subscription](/images/pond/v1-fish-subscriptions.svg)

If a fish on one source is interested in all fishes of a given semantics and name on all devices in
the swarm, it declares the subscription without specifying the source. It will then receive the
merged event streams from all matching fishes. This is used for example by a fish that conceptually
lives on all devices and accepts inputs from anywhere in the swarm (consider, for example, a UserFish that knows
when a given operator starts and stops working, no matter on which terminal these events are
registered).

It is also possible to leave the fish name unspecified when forming a subscription, which says that
this fish is interested in all events or the given semantics. An example of this kind is a data
connector that transfers all time bookings by all users into some external system like Azure SQL for
BI purposes.

## Fishes reading events

Fishes do not act on their own accord, they only react when something happens: either a command comes in (discussed in the next section) or a new event becomes available.
The primary function of a fish is to accumulate local knowledge — state — from the incoming events it has subscribed to.
One **very important note** is that events become available at each edge device individually, whenever devices can talk to each other and exchange the latest information.
Therefore, if you run the same fish logic with the same subscriptions on different devices, they will receive events at different times or in different order; consequently, the current state computed at each device may temporarily be different.
But Actyx Pond will ensure that eventually — when the device has had the chance to catch up with the latest information — all fishes will have seen the same events and will have computed the same state.

![reading](/images/pond/fish-reading.svg)

The illustration shows that besides the events there is one more input, namely the initial state from which the fish will start before it has seen any events.
The `onEvent` handler is a function that takes the current state and the next event and computes the next state from that.
State can be made observable outside the fish by installing an `onStateChange` handler, essentially a function that selects which part of the fish’s internal state to show to the outside world.

One very important consequence of this computational model is that the computed state can be recreated whenever needed, by replaying the input events.
The state is not the most important part, it is not persisted.
In contrast to a database system that only stores the current state, a fish can be fixed retroactively by removing a bug in the business logic and reprocessing all events again.

:::info Remember
Fishes compute their current state by deterministically applying the subscribed event streams.
:::

## Fishes writing events

The second trigger for a fish’s activity is when it receives a command.
These commands can come from other fishes (sent from their `onCommand` handler), from an external system (e.g. via an HTTP call), or from a human operator of the app.

![emitting](/images/pond/fish-writing.svg)

Every incoming command is interpreted in the context of the current state as derived from all locally known events.
The result of the `onCommand` function is a list of events to be appended to the fish’s event stream.
If the command is not valid in the current state then the list of events may be empty, or it may record the fact that an invalid command was received, whichever is required by the business requirements.

It is important to note that the fish cannot change its state in response to a command alone;
it needs to emit an event, which is written to the event stream and then passed into the `onEvent` handler as usual, where the state can be changed.
This is done in order to make all state changes reliably repeatable after a crash or restart of the app — only events are persistent, commands are not recorded and thus also not replayed.

:::info Remember
Fishes record facts (including environmental observation as well as decisions) by emitting events; fact generation is not deterministic and depends on the currently known and possibly incomplete state.
:::

## Availability vs. Consistency

As we have discussed above, the current state of a fish on an edge device may still be missing information that is available elsewhere in the swarm.
The most distinctive characteristic of the Actyx Pond framework is that it allows this fish to still make progress — process commands and emit events — even though this might lead to inconsistencies for a human looking at the whole system.
This trade-off of favoring availability over consistency is a fundamental one, it is impossible to have both, as is also explained at [ActyxOS and CAP](/docs/os/theoretical-foundation/actyxos-and-cap).

We made this choice because a mission-critical environment like a factory shop-floor is built around this same choice already:
groups of persons are working with machines and material to deliver the required products, independent of other processes ongoing on the shop-floor around them.
In such a collaborative setting it is more important to make progress individually than to ensure that every stakeholder on the factory shop-floor has a consistent view on the overall state.
