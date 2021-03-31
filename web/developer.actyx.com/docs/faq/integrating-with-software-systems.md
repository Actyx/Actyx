---
title: How do you integrate with external software systems?
sidebar_label: Integrating software
---

Write a connector app to connect with external software systems.

_Connector apps_ are a type of ActyxOS app, whose purpose is to connect with equipment or software outside of the ActyxOS swarm. Fundamentally, there is nothing special about them, except that they always run in the Docker Runtime because interaction with the outside world does not happen via a user interface.

![Software systems integrationm](/images/faq/integrating-with-software-systems.png)

As shown above, integrating a software system means writing a connector app that uses the network to communicate with an interface provided by the external system. Given the breadth of enterprise software systems, there are a multitude of different interface types and methods, including:

- File-based data exchange
- SOAP or REST HTTP APIs
- Custom TCP/IP-level interfaces

:::note Using Node-RED
[Node-RED](https://nodered.org/) is a tool that allows you graphically construct data flows. It can be quite useful for such scenarios. Node-Red also provides several protocol-specific connector nodes (e.g. SMB) out-of-the-box.
:::

:::note Virtual machines as edge devices
External software systems often run in virtualized environments. This means that it often makes sense to run ActyxOS and your connector app on a virtual edge device itself, as close as possible to the system you are integrating with. Check out the [Which edge devices does ActyxOS support?](supported-edge-devices.md) FAQ.
:::
