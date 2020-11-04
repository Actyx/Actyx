---
title: ActyxOS on Android
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

## Install ActyxOS on Android

### Device requirements

In order to be able to run ActyxOS on Android, your device needs to meet the following requirements:

- Android 6+
- [Android System Webview](https://play.google.com/store/apps/details?id=com.google.android.webview) Version 70+
- Minimum 2GB RAM
- `x86`, `arm64-v8a` or `armeabi-v7a` [ABI](https://developer.android.com/ndk/guides/abis.html#sa)

You can find a list of devices that we know to work well [here](/faq/supported-edge-devices.md).

:::tip Running *ActyxOS on Android* with a fleet management service
For running *ActyxOS on Android* in production, most users set up a fleet management service like [Workspace One](https://www.vmware.com/products/workspace-one.html).
If you require further assistance with running ActyxOS in production, please feel free to contact us at developer@actyx.io or join our [Discord chat](https://discord.gg/262yJhc).
:::

### Required ports

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

*ActyxOS on Android* is publicly available in the [Google Play store](https://play.google.com/store/apps/details?id=com.actyx.os.android). Just open the Google Play store on your device, search for ActyxOS and install it.

After clicking on the ActyxOS icon on your home screen, ActyxOS starts. While ActyxOS is running, you can also see ActyxOS in your notifications overview.

If you do not have access to the Google Play store, please have a look at [our guide](/docs/os/advanced-guides/actyxos-on-android#installing-actyxos-without-access-to-the-google-play-store) in the Troubleshooting section.

### Check the status of your node

In order to check on its status and interact with the node, you can use the [ActyxOS Node Manager](../../node-manager/overview.md) or, if you prefer a command line tool, use the [Actyx CLI](../../cli/getting-started.md).

<Tabs
  groupId="operating-systems"
  defaultValue="node-manager"
  values={[
    { label: 'ActyxOS Node Manager', value: 'node-manager', },
    { label: 'Actyx CLI', value: 'cli', },
  ]
}>
<TabItem value="node-manager">

Go to the **Status** tab. It should show that your ActyxOS node is reachable and **running**:

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

Congratulations, you have successfully installed <em>ActyxOS on Android</em>! Please note that ActyxOS is **not** operational, as you did not configure it yet. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Where to go next

- [Quickstart](../../learn-actyx/quickstart.md) is a tutorial about ActyxOS with ready-to-use apps and configurations
- [Get started](#get-started-with-actyx-on-android) for a detailed guide on how *ActyxOS on Android* works
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions

## Get started with ActyxOS on Android

### Starting and Stopping ActyxOS

After you click on the ActyxOS icon on your home screen, ActyxOS will start. In your notification overview you can also see whether ActyxOS is misconfigured or operational. If you want to stop ActyxOS on your node, you need to go the your device&nbsp;settings&#x2011;&#x2011;>Apps&#x2011;&#x2011;>ActyxOS and then **Force Stop** ActyxOS.

If you would like to know more about hot to configure nodes, please go to the section [**Configuring nodes** in our guide on Node and App Settings](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes)

:::infoNode and App lifecycles
Depending on the lifecycle stage that your ActyxOS nodes or apps are in, your interaction with it might be limited to certain commands. Please check our guide on [Node and App Lifecycles](/docs/os/advanced-guides/node-and-app-lifecycle) to find out more.
:::

### Automatic restart of ActyxOS

If ActyxOS was running on your Android device, it will automatically restart upon reboot.

On some Android distributions you may need to explicitly permit the Autostart in Manage&nbsp;Apps&#x2011;&#x2011;>ActyxOS.

<details>
  <summary>Expand for an example</summary>

Note that the exact name of the setting depends on your Android vendor.

![Enabling Autostart](/images/os/android-settings-0.jpg "ActyxOS App Settings")

This is another example:

![Enabling Autostart](/images/os/android-settings-1.jpg "ActyxOS App Settings")

</details>

### Starting and Stopping Apps

Apart from starting and stopping apps via the [Actyx CLI](/docs/cli/getting-started), you can launch
them via the app list in the ActyxOS app or via home screen icons that you can create under app
details. Closing them in the app switcher works like for every other Android app.

Starting apps via the Actyx CLI or the ActyxOS Node Manager by default works only when ActyxOS is in the foreground.
To make it work in all cases, you have grant ActyxOS permission to: "Display pop-up windows while running
in the background," "Allow apps to start automatically" or similar.
What exactly the permissions are called depends on the Android vendor. When in doubt, just
enable everything under Manage&nbsp;Apps&#x2011;&#x2011;>ActyxOS&#x2011;&#x2011;>Other&nbsp;permissions.

:::caution Restrctions on starting apps from the background on Andoroid 10+
Android 10 introduced [restrictions on starting apps from the background](https://developer.android.com/guide/components/activities/background-starts). After you started an app with the Actyx CLI, or with the ActyxOS Node Manager, it will therefore only start once you move the ActyxOS app into the foreground.
:::

<details>
  <summary>Expand for an example</summary>

Note that the exact name of the settings depends on your Android vendor.

![Granting permissions to start ActyxOS apps](/images/os/android-settings-2.jpg "ActyxOS App Settings")

</details>

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io

### Installing ActyxOS without access to the Google Play store

After you downloaded the APK, you can install it on your Android device via [`adb`](https://developer.android.com/studio/command-line/adb) (If you don't have adb, check [this installation guide](https://www.xda-developers.com/install-adb-windows-macos-linux/)).

Before you connect your edge device to your development machine, make sure that USB debugging is enabled in the developer options. When you connect both devices for the first time, a popup will appear on your edge device and ask you allow the connection. After you established a connection, run:

```bash
adb install actyxos.apk
axosandroid.apk: 1 file pushed. 24.7 MB/s (89486267 bytes in 3.456s)
pkg: /data/local/tmp/axosandroid.apk
Success
```

You should now see *ActyxOS on Android* on the home screen of your Android device. After clicking on the app, ActyxOS starts. While ActyxOS is running, you can also see ActyxOS in your notifications overview.

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` returns one of the two possible reasons (if you are using the ActyxOS Node Manager, you can find this info in the Status tab):

- **ActyxOS is not reachable.**
This means that ActyxOS is not running on your node. Please click on the ActyxOS icon on your home screen. If ActyxOS is running, you can see it in the notifications overview of your Android device
- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457

### ActyxOS node is not connecting to the ActyxOS Bootstrap Node

ActyxOS on Android is currently not able to resolve DNS names inside MultiAddrs and thus only supports ip4 or ip6 MultiAddrs. For example, if you want to connect to the public ActyxOS Bootstrap Node, you have to set the value

- /ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH

instead of

- /dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH

ActyxOS on Docker supports both formats. We are currently working on a fix for this. Check out our [blog](https://www.actyx.com/news/) or [release notes section](/docs/os/release-notes) for information on our new releases.

## Known issues

### Multiple windows for one app are opened in the Android app switcher

If you stop ActyxOS while one of your apps is running and immediately start ActyxOS again, you will see multiple app windows for the same app. In order to avoid this bug, please close all app windows either before, or after you stop ActyxOS. Should you end up in this state, you can resolve the issue by just closing all app windows manually.
