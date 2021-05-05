---
title: Pond 2.5.0 Released
author: Benjamin Sieffert
author_title: Distributed Systems Engineer at Actyx
author_url: https://github.com/benjamin-actyx
author_image_url: /images/blog/benjamin-sieffert.jpg
tags: [ActyxOS, Release]
---

Today we are glad to announce the release of Pond version 2.5.0 [on npm](https://www.npmjs.com/package/@actyx/pond)!

This release contains a whole new set of functions, which operate on events directly – no Fish required.
Read on for a quick overview.

<!-- truncate -->

In order to get an instance of the new APIs, either call `events()` on your `Pond`,
or initialise directly by calling `EventFns.default()` or `EventFns.of(options)` where the connection parameters are the same as for the `Pond.of` call.

### Querying Known Events

Event streams on Actyx never end: New events keep being appended to them.
By calling `offsets()`, the current event numbers (offets, like in an array) per stream can be retrieved.

The function `queryKnownRange` can then be used to get a finite set of events, where `upperBound` is at most equal to the current `offsets`.
This is useful for periodically reading all new events, and exporting them to another DB.

```ts
let lowerBound = await readUpperBoundFromDB()

const exportEvents = async () => {
  const upperBound = await eventFns.currentOffsets()
  const newEvents = await eventFns.queryKnownRange({ lowerBound, upperBound })
  await commitToDB(newEvents, upperBound)
  lowerBound = upperBound
}

// Once per minute, export all new events
setInterval(exportEvents, 60_000)
```

As the pattern of taking `offsets()` as the `upperBound` is so common,  we offer `queryAllKnown` as a shortcut that automatically fills `upperBound` and returns it with the result:

```ts
let lowerBound = await readUpperBoundFromDB()

const exportEvents = async () => {
  const result = await eventFns.queryAllKnown({ lowerBound })
  await commitToDB(result.events, result.upperBound)
  lowerBound = result.upperBound
}

// Once per minute, export all new events
setInterval(exportEvents, 60_000)
```

### Chunking

`queryKnownRangeChunked` and `queryAllKnownChunked` can be used on devices where memory may be unsufficient to hold the full result.

```ts
let lowerBound = await readUpperBoundFromDB()

const exportEventsChunked = async () => {
  lowerBound = await eventFns.queryAllKnownChunked(
    { lowerBound },
    500, // max chunk size
    async chunk => await commitToDB(chunk.events, chunk.upperBound)
  )
}

// Once per minute, export all new events, in chunks of at most 500
setInterval(exportEventsChunked, 60_000)
```

### Indefinitely Subscribing to New Events

`subscribe` can be used to install a callback that will be notified whenever new events become known.
This can be used to dynamically update application state, or simply run a continual export of data:

```ts
let lowerBound = await readUpperBoundFromDB()

eventFns.subscribe(
  { lowerBound },
  async chunk => await commitToDB(chunk.events, chunk.upperBound)
)
```

### Observing the Earliest or Latest Event of a Stream

Often you will be interested just in the latest piece of a certain information, for example the latest state of some machine readings.
Using a `Fish` for that is more complex than needed.

Hence we now offer a dedicated function:
```ts
const MachineTag = Tag('machine')
const CountersChangedTag = Tag<CountersChanged>('counters-changed')

// Keep UI updated with latest readings from the machine
eventFns.observeLatest(
  MachineTag.withId('my-machine').and(CountersChangedTag),
  // Update UI with new information whenever it becomes available
  (newReadings: CountersChanged) => updateUI(newReadings)
)
```

### Unordered Aggregations

One great thing about `Fish` is that events are always fed in a strict order – the same order on every node.  
However, in some cases this is not required: E.g. just summing up a series of integers, the order of those integers does not matter.
Another example is when you are looking for the highest number among a pile of numbers.

We have added the two new functions `observeBestMatch` and `observeUnorderedReduce` as simple shortcuts for these cases.

```ts
const MaterialConsumedTag = Tag<MaterialConsumed>('material-consumed')

// Observe the sum of consumed materials for my-task-id
eventFns.observeUnorderedReduce(
  MaterialConsumedTag.withId('my-task-id'),
  (currentSum: number, consumed: MaterialConsumed) => currentSum + consumed.amount,
  0,
  (newSum: number) => updateUI(newSum)
)
```

### Emitting Events

Finally, there is now a second way to emit events. `emit()` on `EventFns` can be used to directly pass objects with combined tags and payload:
```ts
eventFns.emit([
  {
    tags: ['foo', 'bar'],
    payload: 'my event payload'
  },
  {
    tags: ['foo', 'bar'],
    payload: ['another', 'sort of', 'payload']
  },
])
```

The recommended usage, though, is to still use the typed `Tag` functions:
```ts
const MyTags = Tags<string | string[]>('foo', 'bar')

emit(MyTags.apply('my event payload', ['another', 'sort of', 'payload']))
```

The main advantage of this scheme is that it will properly prevent `Tags<never>` from being attached to anything.

## Closing Words

We hope this new set of function will help to make apps more expressive and powerful – stay tuned for many more new APIs that we have in the pipeline!

<!-- TODO: Link to our detailed docs once they are written -->
