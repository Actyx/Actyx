---
title: Installation
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

ActyxOS is currently available on Android and Docker.
A Beta release for ActyxOS on Windows is also available.

## Installing on Android

_ActyxOS on Android_ is publicly available in the [Google Play store](https://play.google.com/store/apps/details?id=com.actyx.os.android).
Just open the Google Play store on your device, search for ActyxOS and install it.

In order to be able to run ActyxOS on Android, your device needs to meet the following requirements:

- Android 5.1+
- [Android System Webview](https://play.google.com/store/apps/details?id=com.google.android.webview) Version 70+
- Minimum 2GB RAM
- `x86`, `arm64-v8a` or `armeabi-v7a` [ABI](https://developer.android.com/ndk/guides/abis.html#sa)

For further information regarding ActyxOS on Android please start [here](/os/advanced-guides/actyxos-on-android.md).

## Installing on Docker

In order to install ActyxOS on Docker you will need to have a working installation of Docker on your host.
You can find Docker installation instructions [here](https://docs.docker.com/get-docker/).

### Production mode

ActyxOS on Docker is published on [DockerHub](https://hub.docker.com/r/actyx/os). To download and run the latest version in production mode execute the following command.

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/Mac', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker run -it --rm -v actyxos-data:/data --privileged -p 4001:4001 -p 4457:4457 actyx/os
```

:::note
Since `--network=host` is not supported on Windows and Mac we have to explicitly forward the needed network ports.
This is also true of any ports your apps may want to expose, you’d need to add them to this list.
:::

</TabItem>
<TabItem value="unix">

```
docker run -it --rm -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

### Development mode

If you want to run ActyxOS on Docker in development mode, for example because you want to test an ActyxOS application locally without deploying, please use the following command instead. This way all the needed ActyxOS services are also exposed on localhost of the host.

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/Mac', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker run --name actyxos -it --rm -e AX_DEV_MODE=1 -v actyxos-data:/data --privileged -p 4001:4001 -p 4457:4457 -p 127.0.0.1:4243:4243 -p 127.0.0.1:4454:4454 actyx/os
```

:::note
In development mode we additionally need to forward ports 4454 and 4243 to expose the Event Service and the Actyx Pond WebSocket endpoint.
:::

</TabItem>
<TabItem value="unix">

```
docker run -it --rm -e AX_DEV_MODE=1 -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

For further information regarding ActyxOS on Docker please start [here](/os/advanced-guides/actyxos-on-docker.md).

## Installing on Windows

:::caution Beta version only for development purposes
ActyxOS on Windows is currently in public Beta and should not be used in production environments.
:::

You can download an installer for ActyxOS on Windows on [the downloads page](https://downloads.actyx.com/). After opening the installer, you are guided through the setup process.

For further information regarding ActyxOS on Windows please start [here](/os/advanced-guides/actyxos-on-windows.md).

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

## Communicate with the node

In order to check on its status and interact with the node, you need to download the Actyx CLI (`ax` or `ax.exe`) from <https://downloads.actyx.com> and add it to your path.

You can then check on your ActyxOS node:

```bash
ax nodes ls --local <DEVICE_IP>
```

Please refer to the [Actyx CLI documentation](/docs/cli/getting-started) for installation instructions or to learn more about using the Actyx CLI.

:::info
If you want to try out ActyxOS by deploying some sample apps, please take a look at [the Quickstart Guide](../../quickstart.md#run-the-app-in-dev-mode).
:::

## Problems?

Ask for help on [our GitHub repository](https://github.com/actyx/quickstart) or [Twitter](https://twitter.com/actyx) or email developer@actyx.io.

## Learn more

Jump to the different _Guides_ to learn more about the different aspects of ActyxOS.
