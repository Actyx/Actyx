# Banyan tree header

| key | value |
| --- | --- |
| date | 2021-05-05 |
| status | accepted |
| persons | @rkuhn, @rklaehn |

## Decision

We currently don’t have any header at all, which is version 0:

- In this scheme, all keys are all-zeros and we don’t provide signatures.
- Data blocks sent between nodes are just raw Banyan tree blocks.
- The CID in a gossip heartbeat message refers directly to the Banyan tree root.

We will soon implement version 1, which adds a tree creation Lamport timestamp in order to allow ingestion of packed trees:

- In this scheme, all keys still are zero and we don’t provide signatures.
- Data blocks sent between nodes are raw Banyan tree blocks plus a header block.
- The header block contains a `version` field (value: 1), a Lamport `timestamp` (unsigned integer), and a link to the Banyan tree `root`.
- The CID in a gossip heartbeat message refers to such a header block.

## Business Context

Our current customers and deployments are sufficiently well served by

- assuming that a deployment also constitutes a single, contiguous trust domain
- assuming that network transport encryption by swarm key is sufficient
- assuming that we don’t need to treat groups of nodes differently

We foresee that this will change over time.
We will want to distinguish multiple trust domains within a single factory, and we will eventually want to allow events to flow across factory boundaries in a controlled fashion.
We also foresee that we will make further changes to our data storage format, e.g. in terms of encryption at rest.

## Consequences

One particular consequence of the decision is that we will always send the header block with every update.
We consider the resulting uniformity and simplicity of the system to be more important than saving a few bytes.

When adding signatures, we will define a new value for the `version` field.
Headers of this new version will contain the signature bits in addition to the contents of the preceding version.

When adding proper encryption, we will have to refer to the (encrypted) key material from the gossiped block.
For this purpose, we will also define a new value for the `version` field (unless this addition coincides with the addition of signatures).
We will at that point weigh the performance and resource usage to figure out whether to include the key material in the header or put it into a linked block.
