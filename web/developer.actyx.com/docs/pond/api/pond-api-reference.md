---
id: pond-api-full-reference
title: Fish API
permalink: pond/docs/pond-api-full-reference.html
---

Fish full API reference

A Fish is a bundle of the following capabilities:

- storing knowledge, with names and types (semantics + name + onCommand + Events)
- retrieving knowledge (subscriptions + initialState + onEvent)
- modelling entities and their relationships as needed by business processes (the Events and Fishes modeled by the developer)
- active behaviour — decisions being taken by program code (onStateChange)

In this document, we will go through all the parameters of `FishType.of` and provide an in-depth
description of their functionality.

## Introduction

It is important to clarify that all the parameters used to create a Fish are evaluated only in the
local program instance. If a different node initializes a Fish with different parameters, it will
behave differently, even if semantics and name are the same — nodes do **not** coordinate on any
abstract concept of a Fish. Nodes only exchange Events.

A "distributed Fish" is a Fish that subscribes to Events from all nodes — yet a distributed Fish
still has a Fish instance on every individual node, and these instances operate on their own. This
is what enables any node to continue working seamlessly even while disconnected.

Consistency — every Fish instance aggregating its Events into the same State (eventually) — is
achieved by aggregating the same Events, in the same order, on every node. This is the main feature
of the Actyx Pond.

## Semantics

This is a unique string identifier for your Fish Type. It’s used to distinguish Event Streams and is
present in Envelopes via `envelope.source.semantics`.

If two Fish Types have the same Semantics string, that will effectively merge both Fishes in
unforeseen ways! Always make sure that every FishType has a unique Semantics string.

## `onEvent(State, Envelope<Event>) -> State`

The parameters:

- `State` is the hitherto aggregated State of this Fish. For the very first Event the Fish consumes it will be the State given by `initialState(fishName, sourceId).state`; for the second Event, it will be the result of `onEvent(initialState(fishName, sourceId).state, firstEvent)`; and so on.  

- `Envelope<Event>` contains the raw Event data in its `payload` field. The rest is metadata: Where
  did this Event originate, at what time, etc.
  
The return value:

The returned `State` value will be the input parameter for the next application of `onEvent`. It
will also be supplied to `onStateChange` under certain conditions [see there].

`onEvent` is the core of Actyx’ programming model, providing Eventual Consistency in an easily
accessible, out-of-the-box manner: Actyx Pond makes sure that all Events are always passed through
`onEvent` in the **proper order.** If a new Event is received from some peer node, but this Event
actually happened _earlier than the latest_ Event that is part of the current State, the system
automatically restores an earlier State and _replays_ Events on top appropriately.

For a more detailed explanation, see [Time travel](./time-travel)

### Important caveats for implementing onEvent

`onEvent` must be deterministic and side-effect free!

Mutating the input State rather than creating a new one will result in corrupted State! (The input
may however be returned as-is, if no change of State is desired.)

Example:

```typescript
type State = Readonly<{
  foo: number
  bar: number
}>

enum EventType {
  ChangeFoo = 'ChangeFoo',
  ChangeBar = 'ChangeBar',
}

type ChangeFoo = Readonly<{
  type: EventType.ChangeFoo
  newFoo: number
}>

type ChangeBar = Readonly<{
  type: EventType.ChangeBar
  newBar: number
}>

type Event = ChangeFoo | ChangeBar

const onEvent: OnEvent<State, Event> = (state, envelope) => {
  const event = envelope.payload
  switch (event.type) {
    // Never just modify the input `state`!
    case EventType.ChangeFoo: {
      // Either copy the old state, by using triple-dot ("destructuring assignment") syntax:
      return {
        ...state,
        foo: event.newFoo
      }
    }

    case EventType.ChangeBar: {
      // Or explicitly create a new complete object:
      return {
        foo: state.foo
        bar: event.newBar
      }
    }
  }
}
```

To preserve Eventual Consistency, `onEvent` must also be a _pure_ function from input to output. If
you read any other program state in order to decide on the return value, your States will probably
diverge between different nodes!

Example:

```typescript
const onEvent: OnEvent<State, Event> = (state, envelope) => {
  const event = envelope.payload
  switch (event.type) {
    case EventType.ChangeBar: {
      return {
        ...state,
        // Correct: Use only data from the Envelope or the Event!
        bar: event.newBar * envelope.timestamp
      }
    }

    case EventType.ChangeFoo: {
      return {
        ...state,
        // Will diverge between nodes, because it yields a different result each time!
        // There is also no guarantee whatsoever that current Date is close to Event date!
        foo: event.newFoo * Number(new Date())
      }
    }
  }
}
```

## `InitialState(FishName, SourceId) -> (State, Subscriptions)`

Input parameters:

- `fishName`: Name of the Fish being initialized. Useful if you want to subscribe only to Events
  from this very Fish, as opposed to all Fish of its Type.
  
- `sourceId`: ActyxOS node (“event source”) that this Fish lives on. For obvious reasons, on a given node, all Fishes will
  always have the same `sourceId`. If a Fish has a different sourceId, it lives on another node. The
  main application of this parameter currently is to distinguish subscriptions to only local Events
  from global subscriptions. Global subscriptions are created by not supplying a `sourceId`
  parameter to `Subscription.of`, and will listen to Events from all nodes within the swarm. Global
  subscriptions are almost always what you want, since you’re building a distributed system.
  
Output values:

- `state`: The State this Fish is in before reading the first Event.

- `subscriptions`: List of Event Streams that will be merged and passed through the `onEvent`
  state aggregation function.
  
If different nodes ingest different Event Streams, or start with different States, obviously they
will end up in different States. If you are basing the return value on the given `sourceId` in any
way, you are making this very clear: This Fish will behave differently on every node. Most commonly
this is used to put local logic on top of a distributed Event Stream, e.g. marking messages as read
on the very device running the code.

```typescript
type MessageReaderFishEvent = MessageRead | MessageFishEvent

type MessageReaderFishState = Readonly<{
  unreadMessages: Record<MessageId, Message>
  readMessages: ReadonlyArray<Message>
}>

const messageReaderFishOnEvent = (state, envelope) => {
  const event = envelope.payload
  switch (event.type) {
    case EventType.MessageRead: {
      const { [event.messageId]: readMessage, ...unreadMessages } = state.unreadMessages

      return {
        unreadMessages,
        readMessages: [readMessage, ...state.readMessages]
      }
    }

    case MessageFishEventType.Message: {
      return {
        ...state,
        unreadMessages: {
          ...state.unreadMessages,
          [event.messageId]: event.message
        }
      }
    }
  }
}


const messageReaderFishInitialState: InitialState<State> = (fishName, sourceId) => ({
  state: {
    unreadMessages: {}
    readMessages: []
  },
  subscriptions: [
    // Listen to all Events from all MessageFish instances everywhere
    Subscription.of(MessageFishType),
    // Only listen to Events of MessageReaderFish sent by this very source,
    // in order to keep track of local read/unread state!
    Subscription.of(MessageReaderFishType, fishName, sourceId),
  ],
})
```

## `SemanticSnapshot(Envelope<Event>) -> boolean`

A "Semantic Snapshot" is an Event that snapshots the Fish State in a semantic sense; you could also
say that it "resets" the State. The Fish’s State after a Semantic Snapshot is
`onEvent(initialState, semanticSnapshotEvent)` — all Events that sort before the Semantic Snapshot
aren’t relevant anymore.

If you let the Actyx Pond know about such Events via the `isSemanticSnapshot` Fish Parameter, then
it can apply special optimizations. It will also force reset to the Initial State before applying a
Semantic Snapshot Event, so you don’t have to do it yourself in `onEvent`.

For an example usage, see [Semantic Snapshots](./snapshots)

## `OnCommand(State, Command, Event) -> Event[]`

OnCommand is where Events are created. A Command is a user's intent to change the system system
state, which exists only for a short time, on the node it is created on. Fed to its target Fish via
`pond.feed`, it is passed on into `onCommand`, where the program logic decides whether to create any
number of Events from it. A Command may also be 'rejected' by your business logic, simply by turning
it into 0 Events (an empty array).

Input parameters:

- `state`: The currently known Fish State on this node. It may not be the "actual true" State across
  the complete swarm. See below.
  
- `command`: The Command that was fed. Commands are not persisted in any manner, and hence may
  contain functions, circular references etc., so long as these are not propagated into the Events.
  
The result type is `CommandApi`, which you should think of as just an array of Events.

### An important note on trying to use onCommand for validation

There may be _missing_ Events from the past, which haven’t yet reached this node while the Command
is being handled. If they eventually arrive, the Command will **not** be handled a second time with
the updated information — but the late Events can _still_ end up being inserted _before_ the Events
that were returned by `onCommand`.

This will thwart any attempt to ensure with complete certainty: "Event X must not happen when in
State Y." In fact, a fish lives under the same constraints as everyone else, taking decisions
based on possibly incomplete knowledge and then having to make amends or apologies
later, if the decision turns out to have been wrong.

#### The most important feature of onCommand

While you do not have any global guarantees in regards to the State that `OnCommand` sees, there is
a very important local one: Every Command will see the effects (Events) of all previous Commands _on
this node_ incorporated into the `State` already. This can be used to keep things from triggering
twice, locally.

## `OnStateChange(PondObservables<PrivateState>) -> Observable<StateEffect<PrivateState, Command, PublicState>>`

OnStateChange allows installing effects on State changes.

### Publishing State

The most important such effect is publishing the State so that it can be accessed via
`pond.observe`.

In order to just expose the State as it is, use `OnStateChange.publishPrivateState()`.

You can also apply a conversion on the (private) State turning it into some sort of more generally
useful (public) representation via `OnStateChange.publishState(mapper)`. If you are doing this, the
convention is to distinguish the different State representations by calling them `PrivateState` and
`PublicState`.

```typescript
// Imagine a State with an important public part and a complicated internal part:
type PrivateState = Readonly<{
  importantPublicNumber: number
  // Complicated thing only used by the fish internally
  internalBookKeeping: SomeComplexInternalType
}>

// And we want to simply expose just the important number publicly
type PublicState = number

// Then we can use this as onStateChange:
const onStateChange: OnStateChange<PrivateState, PublicState> =
    OnStateChange.publishState((state: PrivateState) => state.importantPublicNumber)
```

#### Creating Commands from State Changes

The more advanced capability of `OnStateChange` is to create a "feedback effect" of sorts that
derives new Commands from State Changes — obviously these Commands may turn into Events, which
result in further State Changes.

To achieve this, you need to pass a custom function as `onStateChange` parameter.

This function receives as input: `PondObservables<Self>`, which offers two functions:

- `observeSelf`: returns an `Observable<PrivateState>`

- `observe`: The normal `pond.observe` which you can use to observe the PublicState of any Fish.

he expected output is `Observable<StateEffect>`. A `StateEffect` can be:

- `StateEffect.sendSelf(command)`: Send a Command to self (will be passed to onCommand the usual way)

- `StateEffect.publish(state)`: Publish some state as public state

For example, we can write `OnStateChange.publishPrivateState()` in full:

```typescript
const publishPrivateState: OnStateChange<PrivateState, Command, PublicState> =
    (pond) => pond.observeSelf().map(state => StateEffect.publish(state))
```

(Note that this `onStateChange` function itself is not actually called anew whenever the State
changes, but rather, it allows you to install your own pipeline on top of the `observeSelf`
State-change stream.)

Since you have the full power of RxJS(5) Observable at your disposal here, you can always create
multiple effects from one State change:

```typescript
// Using concatMap to put more than one action into the Observable.
const onStateChange: OnStateChange<PrivateState, Command, PublicState> =
  (pond) => pond.observeSelf().concatMap(
    state => {
      return [
        StateEffect.sendSelf(createSomeCommandFromState(state)),
        StateEffect.publish(state)
      ]
    }
  )
```

#### Which States are Observed?

Finally, a note about the States that will actually be observed by `observeSelf`: When waking up a
Fish, it will always emit exactly one State, its latest. No matter whether it ingests 1000 Events or
just 10: They will all be aggregated via `onEvent` and only the last State emitted.

Then, while the program is running and new Events arrive, ActyxOS may batch these Events in any
manner. 5 new Events arriving, from some other node, all at once, will probably be processed by the
Fish in one batch, resulting in not 5, but again only 1 new State emitted by the `observeSelf`
Observable.

Thus, you cannot rely on your RxJS pipeline being run after each individual event and therefore must
keep all state that shall be published or turned into commands within the internal state; one
common pattern is to send self-commands to remove information after it has been digested outside of
the fish.

## `SnapshotFormat<State, SerializedState>`

A `SnapshotFormat` is used to enable the Actyx Pond "Local Snapshot" feature. This is a performance
optimization that saves aggregated States to disk, in order to restore more quickly on application
restarts.

A `SnapshotFormat` has the following fields:

- `version`: The version of your Fish logic. Whenever you change anything that would cause States to
  come out differently — e.g. the `onEvent` logic, or `initialState` —, then all States that have
  been persisted to disk are potentially invalid. The Actyx Pond cannot automatically detect such code
  changes; you have to increase this `version` number in order to tell it explicitly.
  
- `serialize` and `deserialize`: Actyx Pond tries to `JSON.stringify` and later `JSON.parse` your Fish
  State in order to persist it. If the State cannot be trivially converted to and from JSON, because
  it uses special data structures, then you need to employ the `serialize` and `deserialize` functions
  to bring it into a JSONifiable form (`serialize`), and back from that form into your actual State
  form (`deserialize`).
  
If your State is trivially JSON-serializable, you can use the `SnapshotFormat.identity(version)`
shortcut to get a `SnapshotFormat` with `serialize: x => x` and `deserialize: x => x`.
  
Bugs in the Serialization Logic, or forgetting to update the version number on Fish changes, will
cause States to diverge across nodes. If you have to write custom serialization logic, always make
sure it is well-tested.
