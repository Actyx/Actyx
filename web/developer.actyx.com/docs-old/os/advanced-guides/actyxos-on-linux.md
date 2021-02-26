---
title: ActyxOS on Linux
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

:::caution Beta version only for development purposes
Beta versions provide early access to future releases. They are intended for development purposes only as they may change without warning or can be removed entirely from a future release. Beta versions must not be used in production environments. ActyxOS on Linux does currently not support any runtimes for apps.
:::

## Install ActyxOS on Linux

### Edge device requirements

Your edge device must meet the following requirements to install <em>ActyxOS on Linux</em>:

- `amd64`, `aarch64`, `armv7` or `arm` architecture

## Required ports

ActyxOS currently requires five ports to operate.
Before starting ActyxOS make sure that these ports are not already in use by another program.

- `4001` - Used for internode communication
- `4243` - Exposes a WebSocket endpoint for the Actyx Pond (only on localhost)
- `4454` - Exposes the [Event Service](/os/api/event-service.md) (only on localhost)
- `4457` - Exposes the [Console Service](/os/api/console-service.md)
- `8080` - Exposes an [IPFS Gateway](https://docs.ipfs.io/concepts/ipfs-gateway/) (only on localhost)

The following ports are reserved for future use.
The services exposed there are currently still in alpha testing.

- `4455`
- `4458`

### Install ActyxOS on your edge device

You can download the binary for ActyxOS on Linux on [the downloads page](https://downloads.actyx.com/). For ActyxOS to start, just run the binary file.

If you have problems with installing ActyxOS, please check our [Troubleshooting section below](#troubleshooting).

### Check the status of your node

In order to check on its status and interact with the node, you can use the ActyxOS Node Manager or, if you prefer a command line tool, use the [Actyx CLI](../../cli/getting-started.md).

<Tabs
  defaultValue="node-manager"
  values={[
    { label: 'ActyxOS Node Manager', value: 'node-manager', },
    { label: 'Actyx CLI', value: 'cli', },
  ]
}>
<TabItem value="node-manager">

Go to the **Status** tab, and you should that your ActyxOS node is reachable and **running**:

![status](/images/node-manager/node-manager-status-1.png)

</TabItem>
<TabItem value="cli">

```text
ax nodes ls --local <DEVICE_IP>
```

You should see something like:

```text
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| NODE ID       | DISPLAY NAME | STATE   | SETTINGS | LICENSE | APPS DEPLOYED | APPS RUNNING | STARTED                   | VERSION |
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| 192.168.2.107 |              | running | invalid  | invalid | 0             | 0            | 2020-03-25T09:32:07+00:00 | 1.0.0   |
+---------------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
```

</TabItem>
</Tabs>

Congratulations, you have successfully installed <em>ActyxOS on Linux</em>! While you can already use ActyxOS locally, you may have to – depending on your exact setup – configure it to be able to connect to other ActyxOS nodes. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Where to go next

- [Quickstart](../../learn-actyx/quickstart.md) is a tutorial about ActyxOS with ready-to-use apps and configurations
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io

### ActyxOS is not starting

The most common reason for this problem is that one of the ports ActyxOS needs is already in use by another program. ActyxOS currently uses the following ports:

- 4001
- 4243
- 4454
- 4457
- 8080

As port 8080 is sometimes already in use by other programs, you can change it by setting the `ACTYXOS_IPFS_NODE__GATEWAY_PORT` environment to a different port.

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` and the returns one of the two possible reasons (if you are using the ActyxOS Node Manager, you can see this info in the Status tab):

- **ActyxOS is not reachable.**
This means that ActyxOS is not running on your node. Please click on the ActyxOS icon on your home screen. If ActyxOS is running, you can see it in the notifications overview of your Linux device
- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457
