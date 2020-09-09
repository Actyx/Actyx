---
title: Installation
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

ActyxOS can be installed on either Docker or Android.

## Installing on Android

_ActyxOS on Android_ is [publicly available in the Google Play store](https://play.google.com/store/apps/details?id=com.actyx.os.android). Just open the Google Play store on your device, search for ActyxOS and install it.

In order to run ActyxOS on your Android device, it must:

- run Android 6.0 or above
- have at least 2GB of RAM

## Installing on Docker

In order to install ActyxOS on a Docker host you will need to have a working installation of Docker (see [the installation documentation](https://docs.docker.com/install/)).

ActyxOS is published on [DockerHub](https://hub.docker.com/), so start the image as follows:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/Mac', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```bash
docker run -it --rm -v actyxos-data:/data --privileged -p 4001:4001 -p 4457:4457 -p 4243:4243 -p 4454:4454 actyx/os
```

:::note
Since `--network=host` is not supported on Windows or Mac we have to explicitly expose the needed network ports.
This is also true of any ports your apps may want to expose, you’d need to add them to this list.
:::

</TabItem>
<TabItem value="unix">

```bash
docker run -it --rm -v actyxos-data:/data --privileged --network=host actyx/os
```

</TabItem>
</Tabs>

ActyxOS should now be running in your Docker environment.

## Required ports

ActyxOS currently requires five ports to operate.
Before starting ActyxOS make sure that these ports are not already in use by another programn.

- `4001` - Used for internode communication
- `4243` - Exposes a WebSocket endpoint for the Actyx Pond
- `4454` - Exposes the [Event Service](/os/api/event-service.md)
- `4457` - Exposes the [Console Service](/os/api/console-service.md)
- `8080` - Exposes an [IPFS Gateway](https://docs.ipfs.io/concepts/ipfs-gateway/)

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
