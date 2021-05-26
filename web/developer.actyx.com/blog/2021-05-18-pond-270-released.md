---
title: Pond 2.7.0 Released
author: Benjamin Sieffert, Wolfgang Werner
author_title: Distributed Systems Engineer, Developer Advocate
author_url: https://www.actyx.com/about
tags: [ActyxOS, Release]
---


We're happy to announce we've released Pond version 2.7.0 [on npm](https://www.npmjs.com/package/@actyx/pond)!

Prior to this version, the pond takes snapshots every 1024 events, provided the first event to include in the snapshot is older than an hour.
This threshold can now be configured as Pond default and for individual fish; the default behavior matches the one in previous versions.

<!-- truncate -->

### Snapshot thresholds

Snapshots are used to alleviate the cost of computing fish states for large numbers of events: Instead of having to look at all events of a fish's subscription, only the latest snapshot and subsequent events need to be taken into consideration. Up until now, the number of events that needed to be present for a snapshot to be created by the Pond was fixed. Now, this threshold can be configured as default for the Pond and/or for individual fish.

Lowering the threshold is especially useful, if a large number of fish, each subscribing to many events, are used in one application. A lower threshold leads to fewer events having to be considered when computing state, thus potentially reducing application initialization lag.

For additional information on snapshots, when to change thresholds, and the implications thereof, please review the [corresponding guide in the documentation](https://developer.actyx.com/docs/how-to/actyx-pond/guides/snapshots).

### Additional improvements

Since setting a lower threshold for snapshot capture may lead to snapshots being captured more often than before, `v2.7.0` also comes with various performance improvements for this under the hood.

Event metadata now contains two additional properties: the `sourceId` and the `offset`, identifying the node emitting the event and the offset in its event log. These properties can be used as upper or lower bound in the form of an [`OffsetMap`](https://developer.actyx.com/docs/reference/pond/modules#offsetmap) in the Pond's [event functions](https://developer.actyx.com/docs/reference/pond/interfaces/eventfns).

### Summary

In most cases, you don't need to adjust snapshot threshold defaults. However, if your specific use case warrants other thresholds, you can change them now for the whole pond and individual fish. Taking snapshots has become more efficient under the hood. Offsets with regard to their source's event log
are now part of the event's metadata.
