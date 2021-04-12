# Unit, Network, and Integration Tests

| key | value |
| --- | --- |
| date | 2021-04-12 |
| status | proposed |
| persons | @rkuhn, @o1iver |

## Decision

We employ the following levels of testing:

- unit tests (within or next to the code module under test)
- network tests (as separate code modules next to the modules under test)
- integration tests (external to tested artifacts)

While unit and network tests are written in the same language as the modules they test, integration tests are written in a scripting language to drive the artifacts.

**Unit tests** cover the logic of individual functions within a module, they serve as progress meters during implementation and aid in identifying the source of network and integration test failures.
Unit tests frequently are written using property-based testing to cover a multitude of scenarios with little code that is easy to read.

**Network tests** run several distributed nodes within a simulated environment to ascertain that given scenarios are handled correctly.
The simulation layer may reside in process (e.g. with in-memory connection between libp2p swarms) or in an external process (like for netsim-embed).

**Integration tests** serve the double purpose of being _acceptance tests_ for all delivered product features (also guarding against regressions) as well as ensuring compatibility with all supported platforms and among our product components (like `actyx`, `ax`, and the SDK).
Integration tests are the only tests that are executed on all host platforms, not just on Linux or the engineer’s host OS and architecture.

## Business Context

We want to confidently release high-quality software to our end-users.
For engineers this entails a comfortable amount of unit tests as well as benchmarks where needed, as well as network tests for higher-level “unit behaviour”.
For product managers this entails a comprehensible set of integration test cases show-casing the desired product features to sufficient detail.
For everyone this entails creating and testing binary artifacts on all supported OS and architecture combinations and ensuring communication or migration between different versions.

## Consequences

Writing these tests and keeping them up to date during refactoring (which should almost exclusively affect unit and network tests) requires additional effort that cannot be spent elsewhere.
This effort is uneqivocally justified by the above business context.

Another consequence is that we will need to work together to keep ourselves honest and hold ourselves to the standard we seek to attain.
