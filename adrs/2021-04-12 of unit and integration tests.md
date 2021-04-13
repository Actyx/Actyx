# Unit, Network, and Integration Tests

| key | value |
| --- | --- |
| date | 2021-04-12 |
| status | proposed |
| persons | @rkuhn, @o1iver |

## Decision

We employ the following levels of testing:

- unit tests (within or next to the code module under test)
- (virtual) swarm tests (as separate code modules next to the modules under test)
- integration tests (external to tested artifacts)
- stress and performance tests (external to tested artifacts)

While unit and swarm tests are written in the same language as the modules they test, integration tests are written in a scripting language to drive the artifacts.
Stress and performance tests also drive artifacts from the outside and are written in whatever language makes sense to produce stress and measure correctness and performance.

**Unit tests** cover the logic of individual functions within a module, they serve as progress meters during implementation and aid in identifying the source of swarm and integration test failures.
Unit tests frequently are written using property-based testing to cover a multitude of scenarios with little code that is easy to read.

**Swarm tests** run several distributed nodes within a simulated environment to ascertain that given scenarios are handled correctly.
This kind of test is specific to the kind of product we are building: a distributed middleware or database needs to test the interplay of (virtually) distributed nodes in addition to the node internals.
The simulation layer may reside in process (e.g. with in-memory connection between libp2p swarms) or in an external process (like for netsim-embed).

**Integration tests** serve the double purpose of being _acceptance tests_ for all delivered product features (also guarding against regressions) as well as _ensuring compatibility_ with all supported platforms and among our product components (like `actyx`, `ax`, and the SDK).
Integration tests are the only tests that are executed on all host platforms, not just on Linux or the engineer’s host OS and architecture.
Integration tests incorporate the intersection between product management and engineering, i.e. they are expected to be of equally high interest to these two roles in R&D (in contrast to unit tests, which are mostly of interest to engineers).

Integration tests are sometimes referred to as “end-to-end tests” or “system tests” in other organisations.
We use the term “acceptance test” with the background of product managers being the customers (since our end-users are not our customers and often do not directly drive our features — we mostly use their input to figure out which problem to solve, not how to solve it).

**Stress and performance tests** place the system (usually a predefined swarm in a virtual network) under a defined load pattern and measure certain characteristics.
These tests are used to notice throughput and latency regressions as well as undesirable CPU, memory, and network resource usage.

## Examples

Considering the Actyx CLI’s feature of creating signed app manifests, these are tests we might have:

- unit tests showing that the Rust function for signing produces an object that can then be validated successfully, as well as various failure and corruption cases, to check that the logic is correct
- integration tests that use actual `ax`, signing utility, and `actyx` to check that signatures work across operating systems and CPU architectures (there might be encoding problems etc.)
- performance tests to check that signing takes less than a second (somewhat contrived example, but we may plausibly do this)

There are no plausible swarm tests for this feature.
A feature that will require swarm tests is ensuring event synchronisation between nodes within <1sec under normal conditions and <10sec after healing a network partition.

## Business Context

We want to confidently release high-quality software to our end-users.

For engineers this entails a comfortable amount of unit tests as well as benchmarks where needed, as well as swarm tests for higher-level “unit behaviour”.
A comfortable mix may for example consist of two thirds unit and swarm tests (mix varying per feature) and one third integration tests.

For product managers this entails a comprehensible set of integration test cases show-casing the desired product features to sufficient detail.
The intention is that a passing integration test suite is sufficient to accept that the feature has been correctly implemented — the level of detail is negotiated between everybody involved, it is expected that not all corner cases are represented in integration test suites.

For everyone this entails creating and testing binary artifacts on all supported OS and architecture combinations and ascertaining successful communication or migration between different versions.
Stress and performance tests are also very relevant for everyone in R&D to agree on and codify our non-functional requirements.

## Consequences

Writing these tests and keeping them up to date during refactoring (which should almost exclusively affect unit and swarm tests) requires additional effort that cannot be spent elsewhere.
This effort is uneqivocally justified by the above business context.

Another consequence is that we will need to work together to keep ourselves honest and hold ourselves to the standard we seek to attain.
