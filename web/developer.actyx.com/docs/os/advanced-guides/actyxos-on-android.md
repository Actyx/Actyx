---
title: ActyxOS on Android
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

## Install ActyxOS on Android

### Edge device requirements

For a list of supported devices, please refer to [Supported edge devices](/docs/faq/supported-edge-devices) Your edge device must meet the following requirements to install <em>ActyxOS on Android</em>:

- Android 5.1+
- [Android System Webview](https://play.google.com/store/apps/details?id=com.google.android.webview) Version 70+
- 2GB Ram
- `x86`, `arm64-v8a` or `armeabi-v7a` [ABI](https://developer.android.com/ndk/guides/abis.html#sa)

:::tip Running ActyxOS on Android with a fleet management service
For running ActyxOS on Android in production, most users set up a fleet management service like [Workspace One](https://www.vmware.com/products/workspace-one.html). Please refer to our guide for [Using Workspace One](/docs/os/advanced-guides/using-workspace-one) for more information.
:::

### Install ActyxOS on your edge device

_ActyxOS on Android_ is [publicly available in the Google Play store](https://play.google.com/store/apps/details?id=com.actyx.os.android). Just open the Google Play store on your device, search for ActyxOS and install it.

After clicking on the ActyxOS icon on your home screen, ActyxOS starts. While ActyxOS is running, you can also see ActyxOS in your notifications overview.

If you do not have access to the Google Play store, please have a look at [our guide in the Troubleshooting section](/docs/os/advanced-guides/actyxos-on-android#installing-actyxos-without-access-to-the-google-play-store).

### Check the status of your node

In order to check on its status and interact with the node, you need to download the Actyx CLI (`ax` or `ax.exe`) from https://downloads.actyx.com and add it to your path (for detailed insallation instructions of the Actyx CLI, go [here](/docs/cli/getting-started)).

You can then check on your ActyxOS node:

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


Congratulations, you have successfully installed <em>ActyxOS on Android</em>! Please note that ActyxOS is **not** operational, as you did not configure it yet. If you want to find out more about configuring ActyxOS node, please check our guide about [configuring nodes](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes).

### Where to go next
- [Quickstart](/docs/quickstart) is a tutorial about ActyxOS with ready-to-use apps and configurations.
- [Get started](#get-started-with-actyx-on-android) for a detailed guide on how <em>ActyxOS on Android works</em>.
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues.
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions.
///
## Get started with ActyxOS on Android

### Starting and Stopping ActyxOS
After you click on the ActyxOS icon on your home screen, ActyxOS will start. In your notification overview you can also see whether ActyxOS is misconfigured or operational. If you want to stop ActyxOS on your node, you need to go the your device settings-->Apps-->ActyxOS and then **Force Stop** ActyxOS.

If you would like to know more about hot to configure nodes, please go to the section [**Configuring nodes** in our guide on Node and App Settings](/docs/os/advanced-guides/node-and-app-settings#configuring-nodes) 

:::infoNode and App lifecycles
Depending on the lifecycle stage that your ActyxOS nodes or apps are in, your interaction with it might be limited to certain commands. Please check our guide on [Node and App Lifecycles](/docs/os/advanced-guides/node-and-app-lifecycle) to find out more.
:::

### Automatic restart of ActyxOS
If ActyxOS was running on your Android device, it will automatically restart upon reboot.

### Starting and Stopping Apps
Apart from starting and stopping apps via the [Actyx CLI](/docs/cli/getting-started), you can click on their icons and close them in the app switcher just as with every other Android app.

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to developer@actyx.io

### Installing ActyxOS without access to the Google Play store
For everyone who does not have access to the Google Play store, ActyxOS is also publicly available as an [APK](https://en.wikipedia.org/wiki/Android_application_package). You can download it from https://downloads.actyx.com/.

After you downloaded the APK, you can install it on your Android device via [`adb`](https://developer.android.com/studio/command-line/adb) (If you don't have adb, check [this installation guide](https://www.xda-developers.com/install-adb-windows-macos-linux/)). 

Before you connect your edge device to your development machine, make sure that USB debugging is enabled in the developer options. When you connect both devices for the first time, a popup will appear on your edge device and ask you allow the connection. After you established a connection, run:

```
$ adb install actyxos.apk
axosandroid.apk: 1 file pushed. 24.7 MB/s (89486267 bytes in 3.456s)
pkg: /data/local/tmp/axosandroid.apk
Success
```

You should now see <em>ActyxOS on Android</em> on the home screen of your Android device. After clicking on the app, ActyxOS starts. While ActyxOS is running, you can also see ActyxOS in your notifications overview.

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` returns one of the two possible reasons:
- **ActyxOS is not reachable.**
This means that ActyxOS is not running on your node. Please click on the ActyxOS icon on your home screen. If ActyxOS is running, you can see it in the notifications overview of your Android device.
- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457.

### ActyxOS node is not connecting to the ActyxOS Bootstrap Node
ActyxOS on Android is currently not able to resolve DNS names inside MultiAddrs and thus only supports ip4 MultiAddrs. For example, if you want to connect to the public ActyxOS Bootstrap Node, you have to set the value 

- /ip4/3.125.108.42/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH 

instead of 
- /dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH

ActyxOS on Docker supports both formats. We are currently working on a fix for this. Check out our [blog](https://www.actyx.com/news/) or [release notes section](/docs/os/release-notes.md) for information on our new releases.
