---
title: Creating Swarms
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

## What's a swarm?

An ActyxOS swarm is a mesh of nodes communicating in a local network. A swarm may have 1 to N nodes in it.

![A simple ActyxOS swarm](/images/os/swarm.png)

A swarm is defined by a single property, the so-called **swarm key**. In order to participate in a swarm, a node must have the secret swarm key. The swarm key is a [setting](../advanced-guides/node-and-app-settings.md) that must be set for a node to function correctly.

## Generate a swarm key

You can use the [Actyx CLI](/docs/cli/swarms/keygen) to generate swarm keys:

```
ax swarms keygen
Generating swarm key, it might take some seconds...
L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjZiNjkzNTQzNGM0Yzc5NjY2OTM4NTkzMjM0Njg0MTY5MzA3NzQ1NmU0MjVhMzk2ZDU3NmE3OTRmNTIzMTc3NTk=
```

## Make a node join a swarm

Use the `ax settings set` command to provide a node with the swarm key.

Using the swarm key generated above as an example:

```
ax settings set --local com.actyx.os/general/swarmKey L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjZiNjkzNTQzNGM0Yzc5NjY2OTM4NTkzMjM0Njg0MTY5MzA3NzQ1NmU0MjVhMzk2ZDU3NmE3OTRmNTIzMTc3NTk= localhost
```

:::tip Validation
ActyxOS will automatically validate the format of the swarm key. If you provide an invalid swarm key, the `ax settings set` command will return an error.
:::

Alternatively you can set the swarm key in a settings object that you pass to the node. See the [example node settings](https://github.com/Actyx/quickstart/blob/master/misc/local-sample-node-settings.yml#L5) from the [quickstart guide](../../quickstart.md).

## Remove a node from a swarm

To remove a node from a swarm simply unset the relevant node setting as follows

```
ax settings unset --local com.actyx.os/general/swarmKey localhost
```
