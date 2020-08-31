---
title: ActyxOS on Docker
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

## Install ActyxOS on Docker

### Edge device requirements

For a list of supported devices, please refer to [Supported edge devices](/docs/faq/supported-edge-devices) Your edge device must meet the following requirements to install <em>ActyxOS on Docker</em>:

- [Docker](https://docs.docker.com/) (for ActyxOS to work, you need to be able to run Docker in privileged mode)
- Ability to run `amd64` or `arm64v8` docker images

If you do not have Docker, check the installation guide for your operating system:

- [Linux](https://docs.docker.com/install/)
- [Mac](https://docs.docker.com/docker-for-mac/install/)
- [Windows](https://docs.docker.com/docker-for-windows/install/)

:::tip Use a fleet management service for production
For running ActyxOS on Docker in production, most users set up a fleet management service like [Balena](https://balena.io/). Please refer to the [Using ActyxOS on Docker with Balena](/docs/os/advanced-guides/using-balena) for more information.
:::

### Install ActyxOS on your edge device

ActyxOS is [publicly available on Docker Hub](https://hub.docker.com/r/actyx/os). You can download and run ActyxOS on Docker with the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/MacOS', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker run --name actyxos -it --rm -v actyx-data:/data --privileged -p 4001:4001 -p 4457:4457 actyx/os
```

You used a couple of common flags here:

- `-it` for running interactive processes
- `--rm` to automatically clean up the container and remove the file system when the container exits
- `-v /tmp/actyxdata/:/data` specifies the volumes that are **not** removed and therefore used for persistent storage. These volumes are used for keeping data safe across container restart. Specifically, it stores Installed apps, app's data, and important ActyxOS data such as your license
- `-p 4457:4457 4001:4001` to publish the ports that the ActyxOS on Docker container uses to communicate to the outside
- `--privileged` as <em>ActyxOS on Docker</em> entails running a Docker daemon inside a Docker container. This enables <em>ActyxOS on Docker</em> to create a self-contained environment

:::info Publishing docker ports
Since `--network=host` is not supported on Windows or Mac you have to explicitly expose the needed network ports.
This is also true for any ports your apps may want to expose, so you would need to add them to this list.
Please refer to the [Docker Documentation](https://docs.docker.com/) for more information on how to run Docker containers.
:::

</TabItem>
<TabItem value="unix">

```
docker run --name actyxos -it --rm -v actyx-data:/data --privileged --network=host actyx/os
```

You used a couple of common flags here:

- `-it` for running interactive processes
- `--rm` to automatically clean up the container and remove the file system when the container exits
- `-v /tmp/actyxdata/:/data` specifies the volumes that are **not** removed and therefore used for persistent storage. These volumes are used for keeping data safe across container restart. Specifically, it stores Installed apps, app's data, and important ActyxOS data such as your license
- `--network=host` for the host's network stack inside the container
- `--privileged` as <em>ActyxOS on Docker</em> entails running a Docker daemon inside a Docker container. This enables <em>ActyxOS on Docker</em> to create a self-contained environment

:::info Docker documentation
Please refer to the [Docker Documentation](https://docs.docker.com/) for more information on how to run Docker containers.
:::

</TabItem>
</Tabs>

:::caution Running ActyxOS on Docker without `--network=host`
If your [ActyxOS Bootstrap Node](actyxos-bootstrap-node.md) is not in the same local network as your ActyxOS nodes; and your ActyxOS nodes are running on Docker without `--network=host`, please read [this paragraph in our troubleshooting section](#running-actyxos-on-docker-without-networkhost).
:::

### Check the status of your node

In order to check on its status and interact with the node, you can use the [ActyxOS Node Manager](../tools/node-manager) or, if you prefer a command line tool, use the [Actyx CLI](../../cli/getting-started.md).

<Tabs
  defaultValue="node-manager"
  values={[
    { label: 'ActyxOS Node Manager', value: 'node-manager', },
    { label: 'Actyx CLI', value: 'cli', },
  ]
}>
<TabItem value="node-manager">

Go to the **Status** tab, and you should that your ActyxOS node is reachable and **running**:

![status](/images/os/node-manager-status-1.png)

</TabItem>
<TabItem value="cli">

```
ax nodes ls --local <DEVICE_IP>
```

You should see something like:

```
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| NODE ID       | DISPLAY NAME | STATE   | SETTINGS | LICENSE | APPS DEPLOYED | APPS RUNNING | STARTED                   | VERSION |
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| 192.168.2.107 |              | running | invalid  | invalid | 0             | 0            | 2020-03-25T09:32:07+00:00 | 1.0.0   |
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
```

</TabItem>
</Tabs>

Congratulations, you have successfully installed <em>ActyxOS on Docker</em>! Please note that ActyxOS is **not** operational, as you did not configure it yet. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Where to go next

- [Quickstart](/docs/quickstart) is a tutorial about ActyxOS with ready-to-use apps and configurations
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions

## Troubleshooting

### Environment variables

#### `AX_DEV_MODE`

If you set this environment variable to 1, you can interact with all ActyxOS services from your Host machine. If you are running ActyxOS on Docker on Mac or Windows, that means you are not using `networking=host`, you need to also pass the following to expose the relevant ports:

- `-p 4243:4243` [for the Actyx Pond](/docs/pond/getting-started.md)
- `-p 4454:4454` [for the Event Service](/docs/os/api/event-service.md)

#### `ENABLE_DEBUG_LOGS`

If you set this environment variable to 1, you will see debug logs from ActyxOS in your shell.

### Starting and Stopping ActyxOS

After you start ActyxOS with the appropriate `docker run` command, ActyxOS will start. After running `ax nodes ls --local <DEVICE_IP>`, you should be able to see your ActyxOS node. If you want to stop ActyxOS on your node, you need to either stop the ActyxOS docker container or stop docker.

If you would like to know more about how to configure nodes, please go to the section [**Configuring nodes** in our guide on Node and App Settings](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes)

:::info Node and App lifecycles
Depending on the lifecycle stage that your ActyxOS nodes or apps are in, your interaction with it might be limited to certain commands. Please check our guide on [Node and App Lifecycles](/docs/os/advanced-guides/node-and-app-lifecycle) to find out more.
:::

### Starting and Stopping Apps

You can start and stop apps via the [Actyx CLI](/docs/cli/getting-started)

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` returns one of the two possible reasons (if you are using the ActyxOS Node Manager, you can see this info in the Status tab):

- **ActyxOS is not reachable.** This means that ActyxOS is not running correctly on your node. Try `docker container ls` to check all your running containers. You can start ActyxOS with the `docker run` command. The command is dependent on your host operating system and described in the installation section above for Windows, Mac and Linux

- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457

### ActyxOS nodes not connecting to each other

Your ActyxOS nodes running on Docker are not able connect to each other if

- you are running ActyxOS without `network=host`; and
- your ActyxOS Bootstrap Node is not running in the same local network as your ActyxOS nodes

This is inherent in Docker, as a container has no access to the IP address of its host unless it is running with `network=host`. Therefore, you have to manually configure the address that your nodes are announcing via the `announceAddress` property in the node settings:

```yml
general:
  displayName: My Node
  swarmKey: 4904199ec5e74cc5871cad1ddad4b9e636c9dfcc55269d954dd4048e336b5433
  bootstrapNodes:
    - /ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH
  announceAddress:    # Manually configured addresses to announce
    - /ip4/192.168.1.101/tcp/4001
    # These must be multiaddresses without peer id, i.e. ip4/<YOUR_IP>/tcp/4001
  logLevels:
    OS: WARN
    Apps: INFO
licensing:
  os: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  apps:
    com.example.app1: development
    com.example.app2: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
services:
  eventService:
    topic: My Topic
    readOnly: false
```

If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io
