# Swarm migration procedure: Actyx Shell App -> Actyx v2.1

|  |  |
| --- | --- |
| date | 2021-06-29 |
| status | proposed |
| persons | @mcamou |


## Decision

1. Migrate at least one headless device in the swarm to ActyxOS v1 (in the case
where they are still running `store-cli`)

1. Wait for the events from all devices to be synced to the ActyxOS v1 node(s)

1. Mark the rest of the nodes (tablets and `store-cli` headless devices) as dead
sources

1. Migrate the ActyxOS v1 headless device(s) to Actyx v2 as per the normal
process

1. Update the externally-stored offset maps (PowerBI pipelines, I believe there
are no others)

1. Replace any `store-cli` containers with Actyx v2

1. Deploy any apps that were previously deployed in the Actyx v1 containers
as stand-alone containers. Change the store endpoint to the `actyx` container,
and replace any references to the `ipfs` container appropriately

1. Remove the shell app from all tablets and install Actyx v2 and MWL

1. Copy the relevant parts of the configuration from the `swarms` directory (in
the case of tablets) and the `deviceConfig/balena` directory (in the case of
headless devices running `store-cli`)

1. Configure Actyx and MWL on all devices

1. Verify that everything works

1. Remove the `ipfs` containers from all headless nodes

1. Clean up `Internal-Deployment-Config` (remove the appropriate `swarms`
directory and affected bootstrap bags)

## Business context

The PERI installations are still running with the Actyx Shell App. To reduce
operations overload and for uniformity, they will have to be migrated to Actyx.
All installations are currently on Shell App 2.3.6, which is still using the
embedded go-ipfs node. Therefore, the migration procedure from 1.x to 2.x will
not work in this case.

## Consequences

- The number of sources will approximately double (for the PERI installations
this should not be a problem)
