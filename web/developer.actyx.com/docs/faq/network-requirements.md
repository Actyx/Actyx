---
title: ActyxOS network requirements
sidebar_label: Network requirements
---

A number of network requirements must be met for ActyxOS to work.

The **local-area network** and **wireless network** (if used) must

- Allow UDP unicast connections
- Not have client-isolation enabled
- Not block ports 4001, 4450-4469 and 5001

The **edge devices** must

- Be connected to the same local-area network
- Allow incoming/outgoing communication on ports 4001, 4450-4469 and 5001

:::note One network = one swarm
All devices in an ActyxOS swarm must be connected to the same local-area network.
:::
