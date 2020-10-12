---
title: Introduction
hide_table_of_contents: true
---

import {SectionHero} from '../../src/components/SectionHero.tsx'
import {TwoElementRow} from '../../src/components/TwoElementRow.tsx'
import {ThreeElementRow} from '../../src/components/ThreeElementRow.tsx'
import {StayInformed} from '../../src/components/StayInformed.tsx'
import {DownloadLink} from '../../src/components/DownloadLink.tsx'
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

## ActyxOS and Actyx Pond in 10 minutes

Here is a video that Alex, one of our engineers, made explaining Actyx in 10 minutes.

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

- the [design principles](design-principles.md) behind ActyxOS,
- [installing ActyxOS](getting-started/installation.md),
- how the [basics work](guides/overview.md); or,
- on what [theoretical foundation](theoretical-foundation/distributed-systems.md) ActyxOS is built

Or start reading the [API reference](api/overview.md).

## Stay informed

<StayInformed
    img1={useBaseUrl('images/social/twitter.svg')}
    img2={useBaseUrl('images/social/github.svg')}
    img3={useBaseUrl('images/social/youtube.svg')}
    img4={useBaseUrl('images/social/linkedin.svg')}
/>

## Contact us / Something missing?

If you find issues with the documentation or have suggestions on how to improve the documentation or the project in general, please email us at developers@actyx.io or send a tweet mentioning the @Actyx Twitter account.
