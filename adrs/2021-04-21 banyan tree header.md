# Banyan tree header

| key | value |
| --- | --- |
| date | 2021-05-05 |
| status | accepted |
| persons | @rkuhn, @rklaehn |

## Decision

For now we leave out the tree header completely, and we name this scheme Version 0.
In this scheme, all keys are all-zeros and we don’t provide signatures.
Data blocks sent between nodes are just raw Banyan tree blocks.
The CID in a gossip heartbeat message refers directly to the Banyan tree root.

## Business Context

Our current customers and deployments are sufficiently well served by

- assuming that a deployment also constitutes a single, contiguous trust domain
- assuming that network transport encryption by swarm key is sufficient
- assuming that we don’t need to treat groups of nodes differently

We foresee that this will change over time.
We will want to distinguish multiple trust domains within a single factory, and we will eventually want to allow events to flow across factory boundaries in a controlled fashion.
We also foresee that we will make further changes to our data storage format, e.g. in terms of encryption at rest.

## Consequences

When adding signatures, we will have to wrap the Banyan tree root in a package (the **tree header**) that contains the signature bits.
This can be done in a multitude of ways, but most importantly we can recognise this wrapping by trying to decode a block as CBOR and checking its structure;
we could for example use a dict with a `header` key that contains the protocol version number.

When adding proper encryption, we will have to refer to the (encrypted) key material from the gossiped block.
We will at that point weigh the performance and resource usage to figure out whether to include the material in the header or put it into a linked block.
Notably, the fast path sent over a long-standing inter-node connection does not need to resend this material unless it actually changes.
