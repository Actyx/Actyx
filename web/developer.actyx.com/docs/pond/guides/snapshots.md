---
title: Snapshots
hide_table_of_contents: true
---

_The state of a fish may become expensive to calculate from scratch: snapshots to the rescue!_

Actyx Pond supports two types of snapshots to avoid processing all known events for a fish’s subscription set during wakeup: _state snapshots_ and _semantic snapshots_.

## State snapshots

:::note

State snapshots are also called “local snapshots” since in contrast to semantic snapshots they are bound to a node. This restriction will be lifted in a future version of Actyx Pond for distributed fishes that consume identical subscription sets when instantiated on different nodes.
:::

The chat room fish in our example keeps a list of messages in its state.
In order to keep its wakeup time constant, we can write this list into a snapshot from time to time, so that not the full log needs to be replayed when the app starts.
Instead, Actyx Pond will load the latest stored snapshot and start from there, replaying only events that come after the snapshot.
The best part about this is that also the writing is done by Actyx Pond. This is enabled by default, using `JSON.stringify` to serialize, and `JSON.parse` to deserialize.

:::note
If you're using custom data types, you only need to implement the `toJSON()` method on your state (like e.g. immutable.js provides for you), and then need to provide a custom `deserializeState` parameter in the fish's definition.
:::note

This is of course only possible if we keep the serialized format the same.  For this purpose the `fishId` has a version
number as well. Upon every change to the necessary interpretation of the serialized data format, the version number
needs to be incremented:

```typescript
export const chatRoomFish = {
  // ... same as before
  fishId: FishId.of('ax.example.ChatRoomFish', 'my-room', 1)
}
```

When the Pond sees this fish waking up with the new version, all old snapshots are invalidated and the newly written ones will have the new version number.

By default, Actyx Pond will take snapshots about every 1000 events consumed by the fish.

In addition to the lastest one, the Pond will retain a snapshot each from last week, last month, and last year.
This helps with the occasional very long time travel: it may happen that a node that still has event stored for this fish lies disconnected in a drawer for a month, and when it comes back online it will synchronize these events, leading the fish to time travel to a state before those events.
The state from a month ago will probably no longer be cached in memory, so a full replay is started, taking advantage of any snapshot that is older than the event that caused the time travel.

## Semantic snapshots

Some fishes have events that completely determine the state after applying said event — you could say that such an event resets the state, regardless of what the previous state was.
Consider as an example the ability to wipe the chat room clean with a new event.

```typescript
type ChatRoomEvent = { type: 'messageAdded'; message: string } | { type: 'messagesCleared' }
```

The result of applying the `messagesCleared` event would be the empty array.
Therefore it would not make sense to replay any prior events, their effect would be undone by this event.
Actyx Pond can be informed about this by providing a function that recognises such “reset events”:

```typescript
export const chatRoomFish = {
  // ... same as before
  isReset: event => event.type === 'messagesCleared',
}
```

Actyx Pond takes note of semantic snapshots as they are encountered and will avoid replaying earlier events to save time.

:::note
Whether an event constitutes a semantic snapshot lies in the eye of the beholder: the chat room fish may consider the `messagesCleared` of its event stream as such an event, but another fish listening to the same event stream may not (e.g. if it shall count all messages ever posted to the chat room). Therefore, the semanticSnapshot property is defined by the fish type and not by the event type.
:::

Both kinds of snapshots can be combined within the same fish as well.
