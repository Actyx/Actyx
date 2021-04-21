# Banyan tree header

| key | value |
| --- | --- |
| date | 2021-04-21 |
| status | draft |
| persons | @rklaehn |

## Options

# The banyan tree header

The purpose of the banyan tree header is to store information that changes very infrequently over the lifetime of a banyan tree. This includes information that is needed to decode the tree, such as a version.

We don't expect the version field to change any time soon, since small changes can be done by extending the existing CBOR format. Nevertheless it is good to have a way to migrate to an entirely new tree encoding, e.g. moving away from CBOR entirely in the future.

It also includes cryptographic information that is needed to access the tree. This will be information that can be used to derive the salsa20 key for the banyan keys and values.

We expect the tree header to be relatively small, < 1kb.

# Referencing the banyan tree header

## Options

- banyan tree header contains hash of banyan tree root. Consequences:
  - tree header needs to be rewritten and resent after each tree update, be it event appending, expiry of events, or compaction
  - tree header can contain a signature of the tree content
- banyan tree header sits next to tree root. Consequences:
  - tree header remains unchanged over a long time. Will change only when the salsa20 key is changed or when the banyan tree format changes
  - tree header does not have to be sent every time
  - integrity of updates needs to be ensured by other means, or left out for now

For option 1, the implementation is pretty straightforward. The tree header is the root of the dag.
For option 2, there are two ways to associate a tree header with a tree:
  - create a dag root node that just has a reference to a banyan tree root and the associated tree header, and include that in each update
    - (-) each fast path update will contain an extra ~80 bytes
    - (+) gossipsub dissemination protocol still contains just a single root, so the minimum amount of data is sent via gossipsub, which has amplification
  - extend the gossip protocol to contain a pair (banyan tree root, tree header cid)
    - (-) gossipsub dissemination protocol contains tuples, so gossipsub messages grow significantly
    - (+) fast path messages do not contain the extra ~80 bytes of the dag root

## Decision

## Business Context

The business purpose of the tree header is to allow individual encryption of banyan trees, and to provide for a clean migration path for format changes in the future.

## Consequences