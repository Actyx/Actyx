---
title: Installation
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

ActyxOS can be installed on either Docker or Android.

## Installing on Android

In order to install ActyxOS on an Android device you need:

- a device running Android 6.0 or above with at least 4GB of RAM
- [`adb`](https://developer.android.com/studio/command-line/adb) running on your machine (see [this installation guide](https://www.xda-developers.com/install-adb-windows-macos-linux/))

Download the latest version of the ActyxOS APK from https://downloads.actyx.com and install the APK on your Android device using [`adb`](https://developer.android.com/studio/command-line/adb):

```
adb install axos.apk
```

ActyxOS should now be installed and running on your Android device.

## Installing on Docker

In order to install ActyxOS on a Docker host you will need to have a working installation of Docker (see [the installation documentation](https://docs.docker.com/install/)).

ActyxOS is published on DockerHub, so start the image as follows:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/Mac', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker pull actyx/os
docker run -it --rm -v actyxos-data:/data --privileged -p 4001:4001 -p 4457:4457 actyx/os
```

:::note
Since `--network=host` is not supported on Windows or Mac we have to explicitly expose the needed network ports.
This is also true of any ports your apps may want to expose, you’d need to add them to this list.
:::

</TabItem>
<TabItem value="unix">

```
docker pull actyx/os
docker run -it --rm -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

ActyxOS should now be running in your Docker environment.


## Communicate with the node

In order to check on its status and interact with the node, you need to download the Actyx CLI (`ax` or `ax.exe`) from https://downloads.actyx.com and add it to your path.

You can then check on your ActyxOS node:

```
ax nodes ls --local <DEVICE_IP>
```

Please refer to the [Actyx CLI documentation](/docs/cli) to learn more about using the Actyx CLI.

:::info
If you want to try out ActyxOS by deploying some sample apps, please take a look at [the Quickstart Guide](../../quickstart.md#run-the-app-in-dev-mode).
:::

## Problems?

Ask for help on on [our GitHub repository](https://github.com/actyx/quickstart) or [Twitter](https://twitter.com/actyx) or email developers@actyx.io.

## Learn more

Jump to the different _Guides_ to learn more about the different aspects of ActyxOS.


