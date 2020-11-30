---
title: Introduction
description: A quick introduction into the architecture and design principles of ActyxOS
---

import {SectionHero} from '../../../src/components/SectionHero.tsx'
import {TwoElementRow} from '../../../src/components/TwoElementRow.tsx'
import {ThreeElementRow} from '../../../src/components/ThreeElementRow.tsx'
import {StayInformed} from '../../../src/components/StayInformed.tsx'
import {DownloadLink} from '../../../src/components/DownloadLink.tsx'
import useBaseUrl from "@docusaurus/useBaseUrl";

ActyxOS makes it easy to run distributed applications on multiple nodes. It is a piece of software that allows you to run your own apps on one or more edge devices and have these apps seamlesslesy communicate and share data with each other.

## Features

<ThreeElementRow
    img1={useBaseUrl('images/icons/distribution.svg')}
    img2={useBaseUrl('images/icons/storage.svg')}
    img3={useBaseUrl('images/icons/edge.svg')}
    title1={"Data dissementation"}
    title2={"Distributed data storage"}
    title3={"Edge runtimes"}
    body1={"Share events between nodes in real-time without the need for central servers or messages brokers."}
    body2={"Store events in the mesh network and have them ready for later consumption."}
    body3={"Run web-apps or docker apps on edge devices and easily package, deploy, configure and monitor them."}
    showLinks={false}
/>

Our goal is to allow people to program the naturally dezentralized collaboration of humans and machines.

## Architecture

On a technical level, ActyxOS combines two things:

- a novel database dealing in flexible event streams, fully decentralised and thus available locally on each edge device — you could imagine this part as a message broker where the readers create the topics by their event stream queries, but without any central infrastructure
- the ability to host apps and manage their settings and lifecycle

Your apps thus interact with both a runtime environment and event stream services, both offered by ActyxOS:

![ActyxOS Architecture](/images/os/architecture.svg)

ActyxOS provides five core services and, where applicable, associated APIs or deployment tools. These services either provide stand-alone functionality or allow you to build your own apps.

Refer to the following sections to learn more about the different services:

- Learn about the WebView Runtime and the Docker Runtime in the [Running Apps](../guides/running-apps.md) section
- Learn about the Event Service in the [Event Streams](../guides/event-streams.md) guide
- The Blob Service is still in Alpha, but you can check out it's [preliminary API](/docs/os/api/blob-service)
- Learn how to use the Console Service in the [Logging](/docs/os/api/console-service) and [App Runtimes](../advanced-guides/app-runtimes.md) sections

## Design principles

- **Availability over consistency** - We favor availability of each individual node to consistency of the system as a whole. Your app should always work, even when other nodes are unavailable

- **Deterministic, orthogonal APIs** - APIs should have well defined behaviors and guarantees you can rely on and should be orthogonal; we like tools that do one thing but do that thing well

- **Layered architecture** - We want to seperate concerns between layers. That is why ActyxOS is composed of different services and is completely independent of the [Actyx Pond](../../pond/getting-started.md)

- **Sensible defaults** - Where possible we provide sensible defaults to flatten the learning curve. If you have specific needs, please [reach out](introduction.md#contact-us--something-missing) and we can help you tune ActyxOS to your needs

## ActyxOS and Actyx Pond in 2 minutes

Here is a video that Alex, one of our engineers, made explaining Actyx in 2 minutes.

import YouTube from 'react-youtube';

<div className="embedded-yt-wrapper">
<YouTube
  videoId="T36Gsae9woo"
  className="embedded-yt-iframe"
  opts={{
    playerVars: { autoplay: 0 },
  }}
/>
</div>

## Get started

Want to jump right in? Check out the [Quickstart](/docs/learn-actyx/quickstart). Alternatively, learn more about

- [installing ActyxOS](installation.md),
- how the [basics work](../guides/overview.md); or,
- on what [theoretical foundation](../theoretical-foundation/distributed-systems.md) ActyxOS is built

Or start reading the [API reference](../api/overview.md).

## Stay informed

If you find issues with the documentation or have suggestions on how to improve the documentation or the project in general, please email us at developers@actyx.io or send a tweet mentioning the @Actyx Twitter account.

<StayInformed
    img1={useBaseUrl('images/social/twitter.svg')}
    img2={useBaseUrl('images/social/github.svg')}
    img3={useBaseUrl('images/social/youtube.svg')}
    img4={useBaseUrl('images/social/linkedin.svg')}
/>
