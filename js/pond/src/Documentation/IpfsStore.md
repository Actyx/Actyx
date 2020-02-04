# IPFS store

The ipfs based event store uses ipfs to store the bulk of the events in a very simple format. A small number of events are
stored outside ipfs for a short time.

# Index store

The main purpose of the index store is to serve as an entry point into ipfs. In addition, it provides a table for storing
metadata such as the source id as well as tables for short term storage of events before they are being added to ipfs.

The index store will always be relatively small, since events are regularly stored in ipfs and then deleted from the index
store.

Depending on the platform (nodejs or chrome), different technologies are used to store the index store. On chrome, we are
using indexeddb. On nodejs we are currently using sqlite.

## Tables
- meta: metadata such as the source id
- sendlog: a table with an autoincrement id to assign psns and store events before they are being sent via pubsub  
- recvlog: contains all events from both pubsub and own events before they are stored in ipfs
- roots: maximum psn and pointer into ipfs for each source

# Data types

## IPFS data format

The data format of ipfs nodes is extremely simple. For each source, we store a simple linked list of nodes, where each node as
a reference to a data block containing a simple array of events. Note that we might add different block formats such as
formats upporting compression in the future.

The linked list of blocks format allows some flexibility regarding size of blocks. E.g. you might store recent events in small
blocks to allow for quick dissemination, but merge older blocks in order to keep the total length of the linked list
manageable.

In addition to the ref to the next node and ref to the data block, nodes carry a very minimal amount of metadata.

The data format for data on ipfs is described in detail in [ipfsDagTypes.ts](/pond/ada/store/ipfsDagTypes.ts)

## Pubsub protocol

The pubsub protocol describes the messages that are sent over the IPFS pubsub system. It is documented in detail in
[ipfsPubsubProtocol.ts](/pond/ada/store/ipfsPubsubProtocol.ts).

### Events

Every pond sends all its generated events over pubsub in a regular interval (default 1s). This allows other ponds to process
these events almost immediately, even before they are stored in ipfs. Note that events are only sent once they are safely
persisted in the index store.

Events are stored in the recvlog table and immediately applied to the pond if possible.

### Snapshots

Every pond sends a snapshot containing the maximum psn and the root for every pond that is known to it.

Snapshots are combined with the own roots by selecting the root with the maximum psn independently for each source. Events
from snapshots are retrieved and processed by the pond if they are still needed.

## Internal data types

Internal types related to the IPFS store are described in detail in [ipfsTypes.ts](/pond/ada/store/ipfsTypes.ts). While the IPFS
data format as well as the pubsub protocol need to remain relatively stable and preserve backward compatibility, the internal
data types can undergo rapid changes.

# Caches

## Node list cache

While the event blocks can be relatively large (thousands of events), a linked list node is of small and constant size.
Therefore it is possible to keep a very large number of them in a memory cache. We are using a tree structure (immutablejs
List<T>) to store a tree of nodes for random access.

Typically, when receiving a new root for a source via a snapshot, traversing the linked list from the new root will eventually
arrive at the old root. Since we cache node lists by their root hash, we will then hit the cache, and the number of IPFS dag
get operations will be limited to the number of links added from the old root to the new root.

In some cases (e.g. block compaction or rewriting history) the new root will have no common nodes with the old root. In that
case, the linked list will have to be traversed to the end.

## Block cache

Event blocks are also being cached. But since they are of large and varying size, they are cached via a separate cache.
