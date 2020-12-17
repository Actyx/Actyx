---
title: ActyxOS Bootstrap Node
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The ActyxOS bootstrap node helps your nodes find their peers.

If you are running ActyxOS only on Windows and Android, nodes will connect automatically through mDNS–assuming mDNS is enabled on your devices and local network. If your swarm also consists of ActyxOS nodes running on docker, or your nodes cannot use mDNS, you can use an ActyxOS Bootstrap Node to connect them. It is configured in each node's settings and will automatically connect all peered nodes in the same swarm to each other.

## Host requirements

- amd64 architecture
- [Docker engine](https://docs.docker.com/install/) (If you are using a fleet management service like [Balena](https://balena.io/) like balena, you do not need to install it manually.)

:::tip Running ActyxOS on Docker with a fleet management service
For running ActyxOS on Docker in production, most users set up a fleet management service like [Balena](https://balena.io/).
:::

## Run your ActyxOS Bootstrap Node

The ActyxOS Bootstrap Node is [publicly available on Docker Hub](https://hub.docker.com/repository/docker/actyx/actyxos-bootstrap-node). You can download and run an ActyxOS Bootstrap Node with the following command:

```text
# Start the bootstrap node with your swarm key
docker run --name actyxos_bootstrap_node --rm --env SWARM_KEY=99eac9c0acbbedf9cfdfcbebfa0bdea99d0bde9edf0 -p 4001:4001 -v actyxos-bootstrap-data:/data actyx/actyxos-bootstrap-node
Starting ActyxOS bootstrap node

ActyxOS bootstrap node running

Bootstrap node address: /ip4/<YOUR_IP>/tcp/4001/ipfs/QmQ3iynxmtZUSNF5dvzQQEYhqnB4sqySRej3A2FgiAMBMH

Set the bootstrap node address using the `ax settings set` command on your
ActyxOS nodes (see https://actyx.com/os/docs/node-settings-schema.html for
more information). Replace <YOUR_IP> above with the IP address of this host.

Press Ctrl+C twice to shutdown this bootstrap node.
```

:::warningRunning the ActyxOS Bootstrap Node without a persistent volume
It is highly recommended to use a persistent volume, as you will otherwise have to change the settings of all your ActyxOS nodes in this swarm each time you start the bootstrap node. If you want the node's identity to be preserved between runs, you need add a persistent volume with `-v` for storage of the node's identity. If you do so, the `SWARM_KEY` environment variable will be ignored in subsequent runs.
:::

On subsequent runs, the Swarm key will be taken from the persistent storage:

```text
# Running with a persistent volume, subsequent runs:
docker run --rm -p 4001:4001 -v actyxos-bootstrap-data:/data actyx/actyxos-bootstrap-node
Starting ActyxOS bootstrap node

ActyxOS bootstrap node running

Bootstrap node address: /ip4/<YOUR_IP>/tcp/4001/ipfs/QmQ3iynxmtZUSNF5dvzQQEYhqnB4sqySRej3A2FgiAMBMH

Set the bootstrap node address using the `ax settings set` command on your
ActyxOS nodes (see https://actyx.com/os/docs/node-settings-schema.html for
more information). Replace <YOUR_IP> above with the IP address of this host.

Press Ctrl+C twice to shutdown this bootstrap node.
```

You used a couple of common flags here:

- `--rm` to automatically clean up the container and remove the file system when the container exits
- `-v actyxos-bootstrap-data:/data` specifies the volumes that are **not** removed and therefore used for persistens storage
- `-p 4001:4001` forwards traffic incoming on the host’s port 4001, to the container’s port 4001

:::infoDocker documentation
Please refer to the [Docker Documentation](https://docs.docker.com/) for more information on how to run Docker containers.
:::

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io

### ActyxOS Bootstrap Node does not connect to the right swarm

If you were running an ActyxOS Bootstrap Node before, you need to clear the persistent volume on your machine to run an ActyxOS Bootstrap Node for another swarm. In order to clear the persistent volume, execute the following command:

```text
docker volume rm actyxos-bootstrap-data
```
