---
title: ActyxOS on Windows
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

:::caution Beta version only for development purposes
Beta versions provide early access to future releases. They are intended for development purposes only as they may change without warning or can be removed entirely from a future release. Beta versions must not be used in production environments. ActyxOS on Windows does currently not support any runtimes for apps.
:::

## Install ActyxOS on Windows

### Edge device requirements

Your edge device must meet the following requirements to install <em>ActyxOS on Windows</em>:

- Windows 10
- `amd64` architecture

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

You can download an installer for ActyxOS on Windows on [the downloads page](https://downloads.actyx.com/). After opening the installer, you are guided through the setup process.

:::info ActyxOS Node Manager included
The installer also includes the latest release of the [ActyxOS Node Manager](../tools/node-manager.md).
:::

ActyxOS is automatically started after the installation is finished.

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

Congratulations, you have successfully installed <em>ActyxOS on Windows</em>! Please note that ActyxOS is **not** operational, as you did not configure it yet. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Where to go next

- [Quickstart](/docs/learn-actyx/quickstart.md) is a tutorial about ActyxOS with ready-to-use apps and configurations
- [Get started](#get-started-with-actyx-on-windows) for a detailed guide on how <em>ActyxOS on Windows works</em>
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions

## Get started with ActyxOS on Windows

### Starting and Stopping ActyxOS

ActyxOS is automatically started after the installation is finished. ActyxOS runs as a Windows Service and can therefore be managed through the Windows Services Manager. [This is a guide for opening the Windows Service Manager](https://www.thewindowsclub.com/open-windows-services).

In the Windows Service Manager, you will see that ActyxOS is running. You can now restart ActyxOS, stop it, and start it again:

![Windows Service Manager](/images/os/windows-service-manager.png)

If you would like to know more about how to configure nodes, please go to the section [**Configuring nodes** in our guide on Node and App Settings](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

:::infoNode and App lifecycles
Depending on the lifecycle stage that your ActyxOS nodes or apps are in, your interaction with it might be limited to certain commands. Please check our guide on [Node and App Lifecycles](/docs/os/advanced-guides/node-and-app-lifecycle) to find out more.
:::

### Automatic start of ActyxOS

When your device is started, ActyxOS is automatically started too. You can configure this by right-clicking on ActyxOS in the Services Manager and then changing the `Startup type` in the properties.

![Windows Service Manager 2](/images/os/windows-service-manager-2.png)

### Logging through the Windows Event Viewer

ActyxOS logs can be accessed through the Windows Event Viewer. In the Event Viewer, you can create a custom view filtering for ActyxOS logs and a log level:

![Event viewer custom view](/images/os/windows-event-viewer-custom-view.png)

You can then see all ActyxOS logs in that view:

![Event viewer](/images/os/windows-event-viewer.png)

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io

### Error during installation

If you receive the following error during installation, please close the Windows Event Viewer and press **Retry**:

![Installation error](/images/os/windows-installation-error.png)

### ActyxOS is not starting

The most common reason for this problem is that one of the ports ActyxOS needs is already in use by another program. ActyxOS currently uses the following ports:

- 4001
- 4243
- 4454
- 4457
- 8080

As port 8080 is sometimes already in use by other programs, you can change it by setting the `ACTYXOS_IPFS_NODE__GATEWAY_PORT` environment to a different port. You can do this by following these steps:

1. [Open the Windows Control Panel](https://support.microsoft.com/en-us/help/13764/windows-where-is-control-panel)
2. Go to System and Security
3. Go to System
4. Go to Advanced System Settings
5. Click on Environment Variables on the bottom right
6. Add a new environment variable:

![Windows environment variable](/images/os/windows-environment-variables.png)

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` and the returns one of the two possible reasons (if you are using the ActyxOS Node Manager, you can see this info in the Status tab):

- **ActyxOS is not reachable.**
This means that ActyxOS is not running on your node. Please click on the ActyxOS icon on your home screen. If ActyxOS is running, you can see it in the notifications overview of your Windows device
- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457
