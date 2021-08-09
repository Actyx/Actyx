# A proposed procedure for upgrading an existing ActyxOS swarm to Actyx v2

|         |                                                 |
|---------|-------------------------------------------------|
| date    | 2020-05-25                                      |
| status  | proposed (pending validation by implementation) |
| persons | @rkuhn                                          |

## Decision

Since the point of this exercise is to retain all previously emitted events, we must also foresee migration of externally stored offset maps.
The ability to rewrite an arbitrary offset map requires:

- a function from old node ID to new stream ID
- a function from old offset (per stream) to new offset

The second part can be significantly simplified if we ensure that event offsets stay the same within each node ID’s stream (by filling gaps with tagless `null` events).
Since old events are all permanent, it is natural to place them either into stream number zero or to reserve a dedicated stream number.
In order to respect local event subscriptions, we need to keep each stream at their node of origin, so **every node rewrites its own streams.**

After the rewrite, each node publishes a suitably tagged migration event containing the old node ID.
This allows easy collection of the mapping function needed for the rewrite of externally stored offset maps.
And it allows detection of when the whole migration is complete.

**Important notes:**

- We need to ensure before the Actyx v2.0.0 release that ActyxOS and Actyx v2 can coexist in the same IPFS swarm (i.e. swarm key) using the same gossipsub topic _without accepting each others’ events and without crashing._
- This document does not describe or consider apps; apps are assumed to have been migrated such that they are compatible with Actyx v2 already, and are assumed to be stopped during the migration.
- Migration on Android will be quite a bit more involved since the Actyx APK won’t be able to access the ActyxOS APK’s database files directly; perhaps we’ll need to keep the ActyxOS APK running and pull out the events via the Event Service.

## Business context

We have existing customers using ActyxOS in production and we want to migrate them to Actyx v2 as soon as possible.
This requires a safe, convenient, and professional upgrade procedure to quell objections.
Customers probably expect something along the lines of “install new Actyx version on all devices, wait a bit, then start using it again.”

## Technical context

ActyxOS nodes and Actyx v2 nodes cannot exchange heartbeats nor events, they don’t even use the same discovery mechanism.
Therefore, the above migration procedure will take nodes out of the old swarm when ActyxOS is stopped and put them into an initially empty new swarm as they complete their local migration procedure.
Old events will not be present in the new swarm unless and until their creating node has migrated them.

## Consequences

The upgrade of a deployment requires that all devices from which events are to be migrated must be brought online.
For so-called _dead sources_ we can either lose the events or recreate a node by doctoring a suitable sqlite database (→ create tool for this).
We need to re-add the code for reading old event chains to Actyx v2; preferably all nodes will first be updated to a version with the sqlite IPFS block store.
The procedure will involve

- starting only the Actyx v2 nodes
- waiting for all events to be migrated
- extracting the node ID → stream ID mapping
- migrating all externally stored offset maps and event IDs
- switching on apps on devices
