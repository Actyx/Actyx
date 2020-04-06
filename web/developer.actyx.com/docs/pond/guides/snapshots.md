---
title: Snapshots
---

The state of a fish may become expensive to calculate from scratch: snapshots to the rescue!

Actyx Pond supports two types of snapshots to avoid processing all known events for a fish’s subscription set during wakeup: _state snapshots_ and _semantic snapshots_.

## State snapshots

:::note

State snapshots are currently called “local snapshots” since in contrast to semantic snapshots they are bound to a device. This restriction will be lifted in a future version of Actyx Pond for distributed fishes that consume identical subscription sets when instantiated on different devices.
:::

The chat room fish in our example keeps a list of messages in its state.
In order to keep its wakeup time constant, we can write this list into a snapshot from time to time, so that not the full log needs to be replayed when the app starts.
Instead, Actyx Pond will load the latest stored snapshot and start from there, replaying only events that come after the snapshot.
The best part about this is that also the writing is done by Actyx Pond, we only need to switch on snapshots as a configuration option:

```typescript
export const chatRoomFish = FishType.of({
  // ... same as before
  snapshotFormat: SnapshotFormat.identity(1)
})
```

The `identity` helper in this case denotes that our State is already JSON serializable and
deserializable without problems. If we were using any sort of custom data storage class, we would
need to pass a custom `SnapshotFormat` with functions `serialize` and `deserialize` that convert to
and from a JSON-stringifiable format.

This is of course only possible if we keep the serialized format the same.
For this purpose the snapshot has a version number as well that we have set to 1 here.
Upon every change to the necessary interpretation of the serialized data format, the version number needs to be incremented.
When that happens, all old snapshots are invalidated and the newly written ones will have the new version number.

With this configuration Actyx Pond will take snapshots about every 1000 events consumed by the fish.

Why is it necessary to keep multiple snapshots?
It may happen that a device that still has event stored for this fish lies disconnected in a drawer for a month, and when it comes back online it will synchronize these events, leading the fish to time travel to a state before those events.
The state from a month ago will probably no longer be cached in memory, so a full replay is started, taking advantage of any snapshot that is older than the event that caused the time travel.

## Semantic snapshots

Some fishes have events that completely determine the state after applying said event.
Consider as an example the ability to wipe the chat room clean with a new command.

```typescript
type ChatRoomCommand = { type: 'addMessage'; message: string } | { type: 'clearAllMessage' }
type ChatRoomEvent = { type: 'messageAdded'; message: string } | { type: 'messagesCleared' }
```

The result of applying the `messagesCleared` event would be the empty array.
Therefore it would not make sense to replay any prior events, their effect would be undone by this event.
Actyx Pond can be informed about this by enabling the semantic snapshot configuration option when defining the fish type:

```typescript
export const chatRoomFish = FishType.of({
  // ... same as before
  semanticSnapshot: (_name, _sourceId) => event => event.payload.type === 'messagesCleared',
})
```

The supplied function computes an event predicate from the fish name and device source ID.
Actyx Pond will during a replay search backwards through the event log, from youngest event to oldest, until an event is found that matches the predicate. This event will then be applied to the initial state of the fish, followed by all succeeding events from the log.

:::note
Whether an event constitutes a semantic snapshot lies in the eye of the beholder: the chat room fish may consider the `messagesCleared` of its event stream as such an event, but another fish listening to the same event stream may not (e.g. if it shall count all messages ever posted to the chat room). Therefore, the semanticSnapshot property is defined by the fish type and not by the event type.
:::

Both kinds of snapshots can be combined within the same fish as well.


