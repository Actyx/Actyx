---
title: ActyxOS on Docker
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

## Install ActyxOS on Docker

### Edge device requirements

For a list of supported devices, please refer to [Supported edge devices](/docs/faq/supported-edge-devices) Your edge device must meet the following requirements to install <em>ActyxOS on Docker</em>:

- amd64 architecture
- [Docker](https://docs.docker.com/) (for ActyxOS to work, you need to be able to run Docker in privileged mode)

If you do not have Docker, check the installation guide for your operating system:
- [Linux](https://docs.docker.com/install/)
- [Mac](https://docs.docker.com/docker-for-mac/install/)
- [Windows](https://docs.docker.com/docker-for-windows/install/)

:::tip Running ActyxOS on Docker with a fleet management service
For running ActyxOS on Docker in production, most users set up a fleet management service like [Balena](https://balena.io/). Please refer to the [Using ActyxOS on Docker with Balena](/docs/os/advanced-guides/using-balena) for more information.
:::

### Install ActyxOS on your edge device

ActyxOS is [publicly available on Docker Hub](https://hub.docker.com/repository/docker/actyx/os). You can download and run ActyxOS on Docker with the following command:

<Tabs
  defaultValue="windows"
  values={[
    { label: 'Windows/MacOS', value: 'windows', },
    { label: 'Linux', value: 'unix', },
  ]
}>
<TabItem value="windows">

```
docker run -it --rm -v actyx-data:/data --privileged -p 4001:4001 -p 4457:4457 actyx/os
```
You used a couple of common flags here:
- `-it` for running interactive processes.
- `--rm` to automatically clean up the container and remove the file system when the container exits.
- `-v /tmp/actyxdata/:/data` specifies the volumes that are **not** removed and therefore used for persistent storage. These volumes are used for keeping data safe across container restart. Specifically, it stores Installed apps, app's data, and important ActyxOS data such as your license.
-  `-p 4457:4457 4001:4001` to publish the ports that the ActyxOS on Docker container uses to communicate to the outside.
- `--privileged` as <em>ActyxOS on Docker</em> entails running a Docker daemon inside a Docker container. This enables <em>ActyxOS on Docker</em> to create a self-contained environment.

:::info Publishing docker ports
Since `--network=host` is not supported on Windows or Mac we have to explicitly expose the needed network ports.
This is also true of any ports your apps may want to expose, you’d need to add them to this list.
Please refer to the [Docker Documentation](https://docs.docker.com/) for more information on how to run Docker containers.
:::

</TabItem>
<TabItem value="unix">

```
docker run -it --rm -v actyx-data:/data --privileged --network=host actyx/os
```

You used a couple of common flags here:
- `-it` for running interactive processes
- `--rm` to automatically clean up the container and remove the file system when the container exits
- `-v /tmp/actyxdata/:/data` specifies the volumes that are **not** removed and therefore used for persistent storage. These volumes are used for keeping data safe across container restart. Specifically, it stores Installed apps, app's data, and important ActyxOS data such as your license.
-  `--network=host` for the host's network stack inside the container.
- `--privileged` as <em>ActyxOS on Docker</em> entails running a Docker daemon inside a Docker container. This enables <em>ActyxOS on Docker</em> to create a self-contained environment.

:::info Docker documentation
Please refer to the [Docker Documentation](https://docs.docker.com/) for more information on how to run Docker containers.
:::

</TabItem>
</Tabs>

### Check the status of your node

In order to check on its status and interact with the node, you need to download the Actyx CLI (`ax` or `ax.exe`) from https://downloads.actyx.com and add it to your path.

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


Congratulations, you have successfully installed <em>ActyxOS on Docker</em>! Please note that ActyxOS is **not** operational, as you did not configure it yet. If you want to find out more about configuring ActyxOS node, please check our guide about [swarms](/docs/os/guides/swarms).

### Where to go next
- [Quickstart](/docs/quickstart) is a tutorial about ActyxOS with ready-to-use apps and configurations.
- [Troubleshooting](#troubleshooting) describes common problems, workarounds and how to get help and submit issues.
- [FAQs](/docs/faq/supported-programming-languages) provides answers to frequently asked questions.

## Troubleshooting

### Getting help and filing issues

If you want to get help or file issues, please write an e-mail to support@actyx.io

### ActyxOS node not responding

First, check that you entered the right IP in the `ax` command. If you still cannot connect, the output of `ax nodes ls` returns one of the two possible reasons:
- **ActyxOS is not reachable.** This means that ActyxOS is not running correctly on your node. Try `docker container ls` to check all your running containers. You can start ActyxOS with the `docker run` command. The command is dependent on your host operating system and described in the installation section above for Windows, Mac and Linux.

- **Host is not reachable.** This means that your development machine cannot connect to your node. Please check that your development machine and your node are in the same network, and your firewall(s) allows them to connect via port 4457.