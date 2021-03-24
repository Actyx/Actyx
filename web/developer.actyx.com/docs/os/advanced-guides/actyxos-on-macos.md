---
title: ActyxOS on macOS
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

:::caution Beta version only for development purposes
Beta versions provide early access to future releases. They are intended for development purposes only as they may change without warning or can be removed entirely from a future release. Beta versions must not be used in production environments. ActyxOS on macOS does currently not support any runtimes for apps.
:::

## Install ActyxOS on macOS

### Device requirements

Your device must run a recent version of macOS (> 10.14 Mojave) to be able to run ActyxOS guaranteed, though earlier macOS versions may work as well. New Apple devices with the ARM-based Apple M1 chip will use [Rosetta](https://en.wikipedia.org/wiki/Rosetta_(software)) to run the binary (an ARM build will be released in the future).

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

### Install ActyxOS on your device

You can download the binary for ActyxOS on macOS on [the downloads page](https://downloads.actyx.com/). For ActyxOS to start, open your terminal, navigate to the directory where you placed the ActyxOS binary and type in the following:

```text
chmod +x actyxos-mac
```

Now, you can simply run the binary with

```text
./actyxos-mac
```

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

![status](/images/os/node-manager-macos.png)

</TabItem>
<TabItem value="cli">

```text
ax nodes ls --local <DEVICE_IP>
```

You should see something like:

```text
+-----------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| NODE ID   | DISPLAY NAME | STATE   | SETTINGS | LICENSE | APPS DEPLOYED | APPS RUNNING | STARTED                   | VERSION |
+-----------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
| localhost | Local Node   | running | valid    | valid   | 0             | 0            | 2021-02-25T07:29:46+00:00 | 1.1.2   |
+-----------+--------------+---------+----------+---------+---------------+--------------+---------------------------+---------+
```

:::caution Additional steps if you use macOS
If you use macOS, you need to additonally allow the Actyx CLI in two steps. First, go to **Settings** and then to **Security & Privacy**. In the **General** tab, you should see a prompt at the bottom that asks you to allow the Actyx CLI. Second, the first time you run an `ax` command, you will be prompted again to allow the Actyx CLI.
:::

</TabItem>
</Tabs>

Congratulations, you have successfully installed <em>ActyxOS on macOS</em>! While you can already use ActyxOS locally, you may have to – depending on your exact setup – configure it to be able to connect to other ActyxOS nodes. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

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
