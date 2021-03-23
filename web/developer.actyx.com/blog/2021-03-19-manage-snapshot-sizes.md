---
title: Snapshot management in Actyx Pond
author: Wolfgang Werner
author_title: Developer Advocate
author_url: https://github.com/wwerner
tags: [Actyx Pond, Event Sourcing, Snapshot]
image: /images/blog/manage-snapshot-sizes/pufferfish.jpg
---

In event-sourced systems, state snapshots are used to alleviate the costs of computing state from event streams. Snapshots are essential to keep processing overhead and latency in check when working with long-lived and/or high traffic models.

The Actyx Pond ships with reasonable defaults for creating and retaining snapshots. However, in certain cases, snapshots may grow too large. This post outlines how to segment state and compress snapshots to avoid this.

<!-- truncate -->

![pufferfish](../images/blog/manage-snapshot-sizes/pufferfish.jpg)

## Recap: Events, State and Snapshots

The state of any given entity in an event-sourced system (a `Fish` in the `Pond`, in our case) at any point in time is defined by the stream of events relevant this entity up to this time. The state is computed by applying these events one by one in chronological order. This means that, the larger the number of events to apply, the more computational resources are required to reach the resulting state.

To prevent having to apply _all_ relevant events each time we want to look at the state, we employ [snapshots](https://developer.actyx.com/docs/pond/guides/snapshots). A snapshot is the persisted result of computing the state for a given point in time. Now, when we look at the state, we don't have to apply all events but only those that happened after the time the snapshot was taken.

Actyx pond transparently manages snapshot creation, persistence and application for you. About every 1000 events, a snapshot is persisted, if the base event is older than one hour. Additionally, the Pond retains snapshots from the past to aid with [longer time travel distances](https://developer.actyx.com/docs/pond/guides/time-travel). 
In case an event leads to the state being completely replaced, you can let the Pond know by returning `true` from the fishes' `isReset` function. This prevents the Pond from unneccessarily going back further in time to compute the state. You can find an example in [Semantic Snapshots](https://developer.actyx.com/docs/pond/guides/snapshots).

So, while the Pond already takes care of a lot of things for you, there still are cases in which you have or want to influence the default behaviour.

## Fish state size considerations

One case that requires special care is if the size of a snapshot exceeds `128MB`. If it does happen, the Pond will let you know by throwing the message `Cxn error: Max payload size exceeded` at you.
While it is uncommon for fishes to grow that large, there are cases in which it might be required. In any case, you should consider the state's estimated size over time in your designs as not to be caught off guard.

In development, you can easily review the sizes of existing snapshots by hooking into the `deserializeState` function and logging it. Just don't leave it enabled in production. State deserialization happens _a lot_.

```ts
const snapshotSize = (snapshot: unknown) =>
    (Buffer.byteLength(JSON.stringify(snapshot)) / 1024 / 1024)
        .toPrecision(4)
    + 'MB'

export const SomeFish = {
    of: (): Fish<S, E> => ({
        // fish details omitted
        deserializeState: (snapshot) => {
            console.debug('Deserializing snapshot of size ' + snapshotSize(snapshot))
            return snapshot as S
        }
    })
}
```

When designing your system, you'll want to model one physical object, process or concept from your problem domain as one fish. This helps you reason and talk about your business domain without having to mentally map additional abstractions. Oftentimes, this quite naturally leads to reasonable sized fish states. With the next version, we're moving to the concept of `local twins` which communicates this 1:1 relationship more explicitly.

### Fat fishes

Two scenarios that tend to lead to large fish states are a) timeseries data and b) exports of aggregated data to external systems like databases for analytics, especially if the target systems are unavailable periodically.

Regarding a), one use case is to visualize sensor logging data.
We'd recommend not to keep timeseries data around in your state for longer periods of time but to push them to external data sinks and flush them from your state once they have been committed. From the external sink, these data can be vizualised using Grafana or similar. Delegate the vizualisation for timeseries to specialized systems and don't implement it yourself in an Actyx application. Not only does this circumvent the size limitation. It also provides you with specialized tooling for vizualisation instead of leaving you on your own to implement this with chart.js, highcharts or even vanilla js + SVG. I've seen this pay off over and over again once changes in charts have been requested.

While exporting to exteral systems is common, the other pattern that can lead to large-ish fish states relates to exactly that. _If_ data from events maps more or less directly to rows in database relations in a 1:1 fashion and _if_ the database is available most of the time, there should be no issues in terms of state size.
But if the state you're looking to export is computed from a larger number of different event types over a larger period of time it may be required to keep more data around to figure out which parts of the database to update. This challenge and solution patterns are described in more detail in [Real-time dashboards and reports made efficient and resilient](https://www.actyx.com/news/2020/6/24/real-time_dashboards_and_reports_made_efficient_and_resilient).

In this case, compressing the fish state's snapshots helps to avoid running into the `128MB` limitation.

## Compressing snapshots

The Pond [documentation](https://developer.actyx.com/docs/pond/guides/snapshots) mentions the possibility of compressing snapshots. Let's walk through implementing it together.

### Evaluating compression

First, we need a suitable compression library. Our own [Benjamin Sieffert](https://github.com/benjamin-actyx) recommends [Pako](https://github.com/nodeca/pako), so we'll stick to that for now. However, there [are](https://github.com/rotemdan/lzutf8.js/) [others](https://pieroxy.net/blog/pages/lz-string/index.html) as well. If you do decide to evaluate them, it would be great if you could share the results.

The following sample explores how to use Pako in isolation and how much it compresses some sample data. To generate a reasonable amount of random data, we use the popular [faker library](https://github.com/marak/Faker.js/). We'll compress and decompress a string and an array of objects, look at the compression ratio and make sure the roundtrip does not mess with our data.

```ts
/*
  package.json:
  "devDependencies": {
    "@types/faker": "^5.1.7",
    "@types/pako": "^1.0.1",
   ...
  },
  "dependencies": {
    "faker": "^5.4.0",
    "pako": "^2.0.3"
    ...
  }
*/

import * as Pako from 'pako' // compression library
import faker from 'faker' // test data generator

const toMb = (size: number) => (size / 1024 / 1024).toFixed(3)

const raw = faker.lorem.paragraphs(50000) // 50k paragraphs of lorem ipsum
const compressed = Pako.deflate(raw) // compress data
const decompressed = Pako.inflate(compressed, { to: 'string' }) // decompress data

const rawO = Array.from({ length: 10000 }, () => faker.helpers.createCard()) // 10k user data objects
const compressedO = Pako.deflate(JSON.stringify(rawO))  // we need to convert our JS to a JSON string for compression ...
const decompressedO = JSON.parse(Pako.inflate(compressedO, { to: 'string' })) // ... and back again
console.table([
  {
    type: 'string',
    rawSizeMB: toMb(Buffer.byteLength(raw)),
    compressedSizeMB: toMb(compressed.byteLength),
    ratio: (Buffer.byteLength(raw) / compressed.byteLength).toFixed(3),
    roundtripOk: raw === decompressed
  },
  {
    type: 'object',
    rawSizeMB: toMb(Buffer.byteLength(JSON.stringify(rawO))),
    compressedSizeMB: toMb(compressedO.byteLength),
    ratio: (Buffer.byteLength(JSON.stringify(rawO)) / compressedO.byteLength).toFixed(3),
    roundtripOk: JSON.stringify(rawO) === JSON.stringify(decompressedO)
  }])
```

This should give us something akin to the following results. We can see that our data is compressed roughly by the factor 3.5. The achievable compression ratio obviously depends on your input data, so I encourage you to run the example on sample data from your application.

|type|rawSizeMB|compressedSizeMB|ratio|roundtripOk|
|---|---|---|---|---|
|string|9.739|2.624|3.711|true|
|object|24.077|6.885|3.497|true|

### Tying it all together

Now that we know how to use the compression library and what to expect from it, let's integrate it into our fish.

As a test scenario, we'll emit an event with the current datetime every few milliseconds and subscribe to it once with and once without compressing the snapshots. After we keep that running for a few hours, we compare the snapshot sizes as described above.

```ts
import { Pond } from '@actyx/pond'
import { CompressingFish, BoringFish } from '../fish'

export type PushEvent = { content: string }
export const pushEventTag = Tag<PushEvent>('pushed')

Pond.default()
  .then(pond => {
    setInterval(() => pond.emit(pushEventTag, { content: Date() }), 150)
    pond.observe(BoringFish.of(), state => console.log('BoringFish has ', state.length + ' items'))
    pond.observe(CompressingFish.of(), state => console.log('CompressedFish has ', state.data.length + ' items'))
  })
  .catch(console.error)
```

The `BoringFish` just aggregates stores all events it receives. We'll keep `deserializeState` from above to track the state's size.

```ts
type State = { data: string[] }

export const BoringFish = {
    of: (): Fish<State, PushEvent> => ({
        fishId: FishId.of('BoringFish', 'Carp', 0),
        initialState: [],
        where: pushEventTag,
        onEvent: (state, event) => {
            return { data: [...state.data, event.content] }
        },
        deserializeState: (snapshot) => {
            console.debug('Deserializing RAW snapshot of size ' + snapshotSize(snapshot))
            return snapshot as State
        }
    })
}
```

In contrast, the `CompressingFish` implements compression using Pako by implementing `deserializeState` in the fish and `toJSON` in the state. `toJSON` will return the compressed data, which might be counter-intuitive. You can think of toJSON() as serialize.

```ts
const pack = (data: any): string => Pako.deflate(JSON.stringify(data), { to: 'string' })
const unpack: any = (zipped: string) => Pako.inflate(zipped as string, { to: 'string' })

type CompressedState = {
    data: string[]
    toJSON: (data: string[]) => {}
}

const INITIAL_STATE = {
    data: [],
    toJSON: () => pack([])
}

export const CompressingFish = {
    of: (): Fish<CompressedState, PushEvent> => ({
        fishId: FishId.of('CompressingFish', 'Fugu', 0),
        initialState: INITIAL_STATE,
        where: pushEventTag,
        onEvent: (state, event) => {
            let data = [...state.data, event.content]
            return {
                data,
                toJSON: () => pack(data)
            }
        },
        deserializeState: (zipped) => { 
            console.debug('Deserializing COMPRESSED snapshot of size ' + snapshotSize(zipped))
            return { data: JSON.parse(unpack(zipped)) } as CompressedState 
        }
    })
}
```

When we keep this running for some time, we should see that ...
* ... both fishes have the same number of items in their state
* ... the size of the compressed snapshot should be significantly smaller than the uncompressed one (well, d'uh!)

And indeed, the logs confirm both assumptions.

```
CompressedFish has 92137 items
Deserializing RAW snapshot of size 6.063MB
BoringFish has 92138 items
Deserializing COMPRESSED snapshot of size 1.002MB
CompressedFish has 92138 items
...
Deserializing COMPRESSED snapshot of size 1.025MB
CompressedFish has  92781 items
Deserializing uncompressed snapshot of size 6.105MB
BoringFish has  92781 items
```

### Cleaning up

Now that we got it working, let's look at the code we've produced. Wrangling `toJSON` into our state in multiple locations is pretty ugly. We mixed up our business code (the state) with technical concerns (serialization). Let's see whether we can do better. Wouldn't it be nice to have a way to make existing fishes compress their state without us having to modify them?

TODO: Add compressing wrapper

Kudos to [Alex](https://github.com/Alexander89) for coming up with this.



## Wrapping up

We looked at some ways to reason about the size of fish states and how influence the way snapshots are persisted.

If you assume you'll be running into the `128MB` snapshot size limitation, you can use the code above to create similar scenario using your own data to validate it. Also, do not hesitate to get in touch. We're always curious to learn how you're using Actyx, what works for you and where your pain points are.

---
Credits: pufferfish photo by [Brian Yurasits](https://unsplash.com/@brian_yuri?utm_source=unsplash&utm_medium=referral&utm_content=creditCopyText)
  