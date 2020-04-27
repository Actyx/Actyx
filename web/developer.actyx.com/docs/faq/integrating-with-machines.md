---
title: How do you integrate with machines?
sidebar_label: Integrating machines
---

To integrate with machines, write a connector app for the [Docker Runtime](../os/advanced-guides/app-runtimes.md#docker-runtime).

_Connector apps_ are a type of ActyxOS app, whose purpose is to connect with equipment or software outside of the ActyxOS swarm. Fundamentally, there is nothing special about them, except that they always run in the Docker Runtime because interaction with the outside world does not happen via a user interface.

In some cases, the machine you are integrating will provide a network-based interface such as OPC UA, MQTT or fieldbuses like PROFINET or EtherCAT. In that case, the architecture is relatively simple: run your connector app in the Docker Runtime on an ActyxOS edge device and interact with the machine over the network.

![](/images/faq/integrating-with-machines-simple.png)

:::note Using Node-RED
[Node-RED](https://nodered.org/) is a tool that allows you graphically construct data flows. It can be quite useful for such scenarios. Node-RED also provides several protocol-specific connector nodes (e.g. OPC UA) out-of-the-box.
:::

If the machine you are integrating does not have any network-based interfaces, you will have to take a slightly more complex approach. In such cases, you must connect _indirectly_ to the I/Os of the PLC or sensors/actuators on the machine.

![](/images/faq/integrating-with-machines-using-digital-io-converter.png)

Since most edge devices don't have I/Os, you use a _Digital I/O Converter_ that sits between the machine's I/Os and the ActyxOS edge device and translates the I/O signals into a network data stream. You then access this stream from your connector app.

:::note W&T Web-IO Connectors
Check out the [W&T Web-IO Connectors](https://www.wut.de/e-50www-10-inus-000.php) if you are looking for a way to translate I/Os to MQTT. They have been successfully used with connector apps running on ActyxOS.
:::




