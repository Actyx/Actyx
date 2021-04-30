# Migration procedure Actyx v1.x→v2.1 for a TypeScript app

|  |  |
| --- | --- |
| date | 2021-04-30 |
| status | proposed |
| persons | @rkuhn, @benjamin-actyx |

## Decision

We assume a TypeScript app packaged for the WebView runtime and using Actyx Pond (in a 2.x version before 2.5.0) or `@actyx/os-sdk`.
The migration procedure has the following steps:

1. migrate from `@actyx/pond` to `@actyx/sdk` using its legacy Pond (API compatible with Pond v2.5 apart from requiring manifest for construction)
2. replace `@actyx/os-sdk` usage with the new SDK’s API for events, logging
3. replace settings access from injected `ax` object to corresponding SDK function
4. deploy the app on Actyx v2 (this document does not describe Actyx node migration)

The important point is that after step 3 the app is compatible with both major Actyx versions.
Whether the resulting app can still interoperate with previously deployed versions and their emitted events depends on how the migration towards tag-based usage of the raw event API has been done.

## Business context

With Actyx v2.0 not yet released and the decision to reach feature parity with ActyxOS v1.x only in Actyx v2.1, our partners are currently building apps on ActyxOS v1.1.
They do so using Pond v2, as instructed by our developer advocates.

We will want all factory deployments of ActyxOS 1.x to be migrated to Actyx v2 within half a year of releasing Actyx v2.1, so that we can benefit from the reduced technology surface area.
Getting our partners and their customers to perform this migration requires that the migration path be reasonably easy and straight-forward.
It would be helped even more if those customers benefited from functional or performance enhancements offered by the new Actyx release.

In order to reduce our engineering effort during the transition we have taken the decision to break all compatibility ties between ActyxOS v1 and Actyx v2:
their network protocols, data storage formats, and service APIs are mutually incompatible.
We provide compatibility on the SDK level instead, also supporting the functionality of the old Pond on the new Actyx version.

## Consequences

- the Actyx SDK (initially only for TypeScript) will need to contain clients for both old and new service APIs
- Actyx v2.1 will need to have all features that are required to support all previous `@actyx/os-sdk` features, including logging and settings
- it is the app author’s responsibility to ensure event log compatibility with old versions by using the `semantics:*` and `fish_name:*` tags if required
- the TypeScript Pond leaks into the Actyx SDK, which may lead to a difference in feature set between the TypeScript and C# SDKs (the latter won’t need to support old Pond APIs)
