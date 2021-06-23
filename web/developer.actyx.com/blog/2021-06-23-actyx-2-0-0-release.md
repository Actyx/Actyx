---
title: Introducing Actyx 2.0
author: Dr. Roland Kuhn
author_title: CTO and co-founder at Actyx
hide_table_of_contents: true
author_url: https://rolandkuhn.com
author_image_url: /images/blog/roland-kuhn.jpg
tags:
  - Actyx
  - 2.0 Release
image: /images/blog/2.0-release/header.png
---

We are incredibly excited to announce our second milestone release – **Actyx 2.0**!

import useBaseUrl from '@docusaurus/useBaseUrl'

<img style={{marginBottom: "12px", borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/header.svg')} />

<!-- truncate -->

ActyxOS version 1.0 – released in September 2020 – was the first generation of the Actyx Platform that we offered to the general public.
It was built on top of 4.5 years of experience with software installations in factories across the globe.
Since its release, we – together with our [partners](https://www.actyx.com/partners) – have had great success digitizing manufacturing processes in factories from a wide range of industries.

With the help of our partners, we could gather valuable insights into the usage of our product and gained a deeper understanding of the requirements and needs of our users.
These learnings ultimately made us rethink every aspect of our product and helped us to formulate a clear goal for the next major release.

Today, we are proud to release the next evolution of ActyxOS 1.0: **Actyx 2.0**!
This release introduces changes to all parts of the product;
from non-user-facing internal upgrades to the core of our technology to user-facing improvements on the API level.

Let's go over four of the biggest changes we introduce with Actyx 2.0:

- Production support for all major platforms
- Performance improvements
- New APIs
- Actyx Node Manager

## Actyx in Production

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/platform-support.svg')} />

With version 2.0, we now officially support Actyx in production on all major platforms: Windows, Linux, macOS, and Android!
This also means that you can replace your Docker setups with their native counterparts which run more stable and are much easier to debug.
Please refer to our [developer documentation](https://developer.actyx.com/docs/reference/actyx-reference) for a detailed summary of supported operating systems and processor architectures.

## Performance Improvements

Actyx 2.0 brings impressive under-the-hood performance improvements.
These gains were achieved by completely redesigning our back-end, changing the way we store and retrieve events on Actyx nodes.
Our team of engineers came up with and implemented a tree data structure called Banyan Tree.
Banyan is a tree that is not very deep but can grow extremely wide.
This allows us to retrieve frequently accessed data _much_ faster than with traditional data structures.
This is useful for two reasons:
first to reduce the size of data that has to be loaded into memory to sift through a tree, and second to allow for efficient in-memory caching of just the frequently accessed data.

<!--TODO insert performance benchmarks -->

:::info Want to know more about Banyan?
Be sure to check our blog in the coming days as we will soon release a blog post about the Banyan implementation that dives into all its amazing, nitty-gritty, technical details.
:::

## New APIs

Another big change we introduce to our users with this release are our new, and redesigned APIs.
Actyx 2.0 brings new APIs as well as improvements in ergonomics in existing APIs.

### Auth API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/auth-api.svg')} />

The Auth API is a new API that is responsible for checking whether users are authenticated and authorized to access the Actyx APIs.
To do that, you need to provide a valid app manifest to the auth API and you will receive a token to be used with HTTP requests to the Events API.

### Events API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/events-api.svg')} />

The Events API is what was formerly known as the Event Service.
It is the core of Actyx and offers users the ability to publish, query, and subscribe to events.
New functionality includes:

- support for tag-based queries instead of name and semantics
- subscribe to events monotonically, i.e. with the guarantee that whenever the service learns about events that need to be sorted earlier than an event that has already been delivered the stream ends with a time travel message

### Node API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/node-api.svg')} />

The Node API is a new API that is responsible for providing access to node-specific information.
Currently, the API returns the ID of the node.
In the future, more endpoints will be added that will give more insights into the state and performance of a node, so stay tuned and remember to check our [public roadmap](https://trello.com/b/thhTs62O/actyx-product-roadmap)!

## Actyx Node Manager

<img style={{boxShadow: "0 20px 38px -8px rgba(80, 80, 100, .5)", marginBottom: "42px", borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/node-manager-ui.png')} />

Actyx Node Manager is an electron app that lets you securely manage and configure decentralized swarms of Actyx nodes in the local network.
With Actyx 2.0 we ship a completely redesigned version of the Node Manager, with better performance and a lot of UX improvements that will significantly streamline working with it.
For instance, Actyx Node Manager now lets you work on multiple nodes at the same time with favorite nodes being saved across sessions so that you can start right where you left off.
For more information on the new Node Manager, please refer to the [reference documentation](https://developer.actyx.com/docs/reference/node-manager.mdx) or the release [changelog](https://developer.actyx.com/releases).

<!-- markdownlint-disable MD025 -->

# Actyx Insider Program

Apart from new features, we are also very excited to announce the **Actyx Insider Program**!

The Actyx Insider Program is a community of Actyx's biggest fans who get to be the first to see what's next.
Actyx Insiders run previews of the platform, then give feedback and engage directly with our Engineers and Product Managers to help shape the future of our product.
Be the first to see what's next for Actyx and join the community and give us your feedback to help make Actyx even better, together.

# Getting Started

For the last months, we worked incredibly hard on this release and we couldn't be more excited to share it with you.
Most of the new features introduced with Actyx 2.0 build a foundation for further improvements and features.
They unlock a roadmap of new features that we are already busy working on.
We have several improvements in progress, such as the Actyx SDK for C# or configuration of ephemeral event streams so be sure to check our [community forum](https://community.actyx.com/), [socials](https://twitter.com/actyx), or [blog](https://developer.actyx.com/blog) so you don't miss any updates.

Now it is your turn to take this new tool and use it to quickly implement all the use-cases you previously wanted to solve but couldn't due to incompatible infrastructure requirements.

To get started, simply [download Actyx](https://developer.actyx.com/releases) and start with one of our [tutorials](https://developer.actyx.com/docs/tutorials/overview).
For a more detailed introduction to Actyx, please check out our [Actyx Academy](https://academy.actyx.com/)!

As always, we are keen on hearing your opinions and feedback.
If you have any questions or requests please visit our [developer community forum](https://community.actyx.com/)!

Happy hacking!
