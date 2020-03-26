---
title: Introduction
---

## What is ActyxOS?

### ActyxOS is a *multi-device operating system*. What does that mean?

Check this [page](../cli.md).

Check out [this page](../pond/design-principles.md).

:::info Hey there
This is an info box.
:::

:::tip Hey there tip
This is an tip box.
:::

:::note Hey there note
This is an note box.
:::

:::warning Hey there warning
This is an warning box.
:::

:::danger Hey there danger
This is an danger box.
:::

Traditional single-device operating systems such as Linux, Microsoft Windows, or Android allow users to run applications on a single device. ActyxOS is an operating system for running distributed applications on multiple devices within a local area or mesh network.

ActyxOS offers four key capabilities:

- Run applications on one or more hardware devices
- Share data between applications across the network
- Offer redundant data storage for later data retrieval
- Monitor, operate and maintain a deployment via the cloud

ActyxOS is built for scenarios where multiple edge devices need to communicate and collaborate. In such scenarios, ActyxOS offers two significant advantages.

- **Truly serverless**

  Applications run on devices in a completely decentralized fashion. There are no central databases or servers necessary for the system to work. This not only eliminates single-points-of-failure, but also removes the need to manage additional infrastructure components.

- **Offline capable**

  Applications always run—irrespective of network partitions or device outages. If a device is disconnected, or the network is congested, the application still works. Once connectivity with other devices is reestablished, ActyxOS automatically synchronizes data between devices and applications.

## How does it work? {#how-does-it-work}

To provide these key capabilities, ActyxOS does several things automatically in the background requiring no or minimal setup, configuration or management.

### Device registration {#device-registration}

![ActyxOS Device registration](../images/device-registration.png)

Once a device has been provisioned and set up in the Actyx Console, ActyxOS automatically sets it up with all necessary registration and security credentials for joining an existing device swarm.

### Peer discovery {#peer-discovery}

![ActyxOS Peer discovery](../images/peer-discovery.png)

ActyxOS automatically discovers other devices in the same registered swarm within a local area network&mdash;even in a dynamic network topology without static IP addresses.

### App runtime {#app-runtime}

![ActyxOS App runtime](../images/app-runtime.png)

The ActyxOS EdgeRT runs apps as configured. The configuration is pulled from the app's manifest or can be overwritten using the Actyx CLI or the Actyx Console.

### Event streaming {#event-streaming}

![ActyxOS Event streaming](../images/event-streaming.png)

Events published by apps are streamed in real-time to subscribed apps. Every subscribed app on any device will eventually receive events even if the network was previously partitioned.

### Event persistence {#event-persistence}

![ActyxOS Event persistence](../images/event-persistence.png)

ActyxOS automatically persists events in a redundant fashion, allowing apps to access not just current and future events, but also access past events published to a specific topic.

### Blob storage {#blob-storage}

![ActyxOS Blob storage](../images/blob-storage.png)

ActyxOS allows apps to store large data blobs in the swarm, where they are automatically distributed for access by other devices and apps.

### Phone home {#phone-home}

![ActyxOS Phone home](../images/phone-home.png)

To facilitate deployments and continuous monitoring, ActyxOS automatically phones home when possible, providing operational metrics and the ability to deploy new or updated apps.

## What can you do with it? {#what-can-you-do-with-it}

ActyxOS is completely domain-independent, meaning you can build and run anything you want with it. For examples of factory solutions built on ActyxOS, check out the following video case studies:

- [CTA GmbH - A complete logistics and production system built on ActyxOS](https://www.youtube.com/watch?v=bZz-hh8GPJc)
- [Stölzle Glass Group - An operator assistance system for glass manufacturing](https://www.youtube.com/watch?v=xPz_p7HSrZA)
- [PERI Group - Digitized workflows and processes for repair and maintenance](https://www.youtube.com/watch?v=k-p9Ze6prsM)

Beyond this, ActyxOS is suitable for almost any factory use-case where multiple devices&mdash;from machine connectors to mobile devices&mdash;need to communicate and collaborate reliably on the edge.

## What can you not do with it? {#what-can-you-not-do-with-it}

We are continuously adding new features and functionality to ActyxOS, but there are some limitations as of now.

### No intra-swarm security {#no-intra-swarm-security}

ActyxOS currently does not secure event streams from access by other apps within the same swarm. This means that any app running in a swarm can access any event stream published by any other app.

We plan to release functionality for securing event streams with a developer certificate in the future. No release date is planned yet.

### No microsecond latency {#no-micro-second-latency}

ActyxOS currently employs standard TCP connections between devices, allowing for typical event dissemination latencies in the range of 10–100ms while devices are connected to the network. Low-latency transport options may be implemented in a future release.

### No high-frequency event rates {#no-high-frequency-event-rates}

ActyxOS currently achieves maximum average event processing rate of **50 events/sec** and will severely deteriorate if events are generated at a higher frequency.

## Where do you go from here? {#where-do-you-go-from-here}

Now that you have understood the basics of ActyxOS, have a look at the following resources to dive deeper into ActyxOS:

- Learn about the [architecture](/os/docs/architecture.html) of ActyxOS
- Jump into the _Main Concepts_ with a [Hello World](/os/docs/hello-world.html) example
- Read through the advanced guides, starting with the [Event Service](/os/docs/event-service.html)