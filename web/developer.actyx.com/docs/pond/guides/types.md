---
title: All those Types
hide_table_of_contents: true
---

We have chosen TypeScript for a reason: fishes offer more type-safety than using the JSON-based untyped Event Service by itself.

The full signature of `Fish` has two type parameters:

- `S` is the type of the state that the fish accumulates by processing events
- `E` is the type of events understood by the `onEvent` handler

In particular, the full type of the chat room fish we have developed so far is

```typescript
const chatRoomFish: Fish<string[], ChatRoomEvent> = ..
```

This allows interactions with fishes via `Pond.run` and `Pond.observe` to be checked for correctness
by the TypeScript compiler.

Via the [typed tagging system](typed-tags), correctness is also checked for `Pond.emit`: Given a set
of tags (constructed like `someTag.and(someOtherTag).and(aThirdTag.withId('some-id'))`), only those
events may be emitted that are part of the type of all of these tags.

On the flipside, a Fish’s `onEvent` is type-checked to cover _at least_ all the event types declared
by its subscription set.

```typescript
const UserTag = Tag<UserEvent>('user')
const MachineTag = Tag<MachineEvent>('machine')

// Requiring either one of the tags means that the fish must have handling for the complete set
const whereUserOrMachine: Where<UserEvent | MachineEvent> = UserTag.or(MachineTag)

// Requiring both tags means the fish must only have handling for
// Types that are common to both Union Types
const whereUserAndMachine: Where<Extract<MachineEvent, UserEvent>> = UserTag.and(MachineTag)
```

:::note
In a future version ActyxOS will support the registration of event schemata for event streams, allowing types to be checked across nodes and apps. This will include compile-time declarations for TypeScript as well as runtime checks for all events passed into the Event Service API. For now, you can use the [typed tag](/docs/pond/guides/typed-tags) query API to gain better type guarantees within the Pond app itself.
:::

Static type information also gives you some measure of control over the evolution of your event types:
when changing the definition of the event type, you and your team will see this explicitly so that you can carefully consider whether the changes will be backwards compatible, i.e. whether the changed fish code will be able to still understand the existing old events.

An [event-sourced](https://martinfowler.com/eaaDev/EventSourcing.html) system like ActyxOS needs similar care as a widely used database when updating the data schema.
In our case it is not a table structure whose columns change, it is a set of events whose properties may change, or new events may be added and old ones deprecated.
With the current Actyx Pond infrastructure, it is necessary to retain compatibility with old events when making changes, i.e. old events will stay in the event log as they were and will still need to be understood by new app versions.

:::note
In a future version ActyxOS and Actyx Pond will support the registration of schema migration handlers that transform an event stream from one schema version to another. With this, the app code can be modified to work with the new types and the translation of old events is done by the infrastructure, splitting the management of backwards compatibility from the evolution of the program code.
:::

The next section addresses another concern that arises from the event-sourced nature of ActyxOS:
with an ever-growing event log, waking up a fish would take longer the longer it has existed.
This is addressed using snapshots.
