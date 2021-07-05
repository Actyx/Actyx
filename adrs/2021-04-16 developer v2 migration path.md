# V2 Migration Path for Developers

| key | value |
| --- | --- |
| date | 2021-04-16 |
| status | accepted |
| persons | @rkuhn, @benjamin-actyx |

## Decision

We make an effort to allow users to update their app to a state where it is compatible with _both_ ActyxOS V1 and Actyx V2.
This serves to help with migrating to Actyx V2, which is by itself not compatible with ActyxOS V1.
Eventually, when a developer has migrated everything to V2, she will reap the ability to work with novel APIs that only work from V2 onwards.

## Business Context

We aim to make migration from V1 to V2 easy for developers.
We also aim to enable developers that are already active on V1 today to start using V2 concepts and APIs asap, i.e. use tags over semantics/name.

To this end, we provide a good level of source code compatibility across the v1→v2 Actyx upgrade by using the Pond library.
This covers many customer cases as it is; users of the raw HTTP API can be served by implementing a slim adapter layer as an HTTP server in node.js that uses the new Pond to talk to the Event Service.

This whole setup is intended as an interim measure to aid our users and customers through the transition period to Actyx v2.

## Consequences

We add V2-style event-APIs to Pond 2.6.0. This release will still only work with V1. This is to get these APIs out ASAP. The code lives on branch `actyx/v1`.
It is integration-tested on that branch against V1.

We introduce a new TS SDK which exposes roughly the same APIs, but completely migrated to V2 terminology.
The most notable impact of that is `sourceId` being called `stream` in the `Metadata`.

This SDK is compatible with both V1 and V2, by detecting which version of Actyx is running, and adjusting the WS communication to that.
This allows for example developing on V2 but deploying to an installation that is still running on V1.
The code lives on `master` and is integration-tested against V2.
The API entrypoint of the SDK requires the developer to supply an Actyx AppManifest to authorize communication with V2.

This new SDK will become a dependency of the Pond, yielding a Pond release that is also compatible with both V1 and V2.
This Pond release marks version 3.0, since there are a few breaking changes:

- Mandatory to pass Actyx Manifest when constructing a `Pond` object
- Node connectivity feature no longer available
- Local snapshots disabled
- `PendingEmission` renamed to `PendingCommand`

## Future

Eventually the SDK will grow to include V2-specific shiny new features, and V1 support will be dropped.
The Pond will probably be discontinued at that point, and replaced with features that live directly in the SDK.
