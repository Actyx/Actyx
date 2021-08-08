---
title: Introducing Actyx 2.0
author: Dr. Roland Kuhn
author_title: CTO and Co-Founder at Actyx
author_url: https://rolandkuhn.com
author_image_url: /images/blog/roland-kuhn.jpg
tags: [Actyx, Release]
image: /images/blog/2.0-release/thumbnail.png
---

We are incredibly excited to announce our second major release – **Actyx 2.0**!

import useBaseUrl from '@docusaurus/useBaseUrl'

<img style={{marginBottom: "12px", borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/header.svg')} />

<!-- truncate -->

ActyxOS version 1 – released in September 2020 – was the first generation of the Actyx Platform offered to the general public.
It was built on top of 4.5 years of experience with software installations in factories across the globe.
Since its release, we – together with our [partners](https://www.actyx.com/partners) – have had great success digitizing manufacturing processes in factories across almost all industries.

With the help of our partners, we gathered valuable insights into the usage of our product and gained a deeper understanding of the requirements and needs of our users.
Based on these learnings we reviewed every aspect of our platform, leading to a much refined experience.

Today, we are proud to release the next step of our evolution: **Actyx 2.0**!
This release introduces changes to all parts of the product,
from internal upgrades under the hood to user-facing improvements on the API level.

Let's go over four of the biggest changes we introduce with Actyx 2.0:

- Production support for all major platforms
- Significant performance and stability improvements
- More ergonomic and powerful APIs
- A more useful and usable Actyx Node Manager

## Actyx in Production – on all major platforms

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/platform-support.svg')} />

With version 2.0, we now officially support Actyx in production on all major platforms: Windows, Linux, macOS, Docker, and Android!
This also means that you can in many cases use native binaries instead of running Actyx in Docker, making it easier to set up peer-to-peer networking.

Please refer to our [developer documentation](/docs/reference/actyx-reference) for a detailed summary of supported operating systems and processor architectures.

And before I forget: if you’re on Windows you’ll _love_ our new MSI installer, which also makes Actyx run as a Windows service.

## Performance improvements from top to bottom

Actyx 2.0 brings serious _under-the-hood_ performance improvements.
These gains were achieved by completely redesigning the core of Actyx, changing the way we store and retrieve events on Actyx nodes.
Our team of engineers came up with and implemented a tree data structure called Banyan Tree.
Banyan is a tree that is not very deep but can grow extremely wide.
This allows us to retrieve frequently accessed data _much_ faster than with traditional data structures.
The benefits of this approach are twofold:
first to reduce the size of data that has to be loaded into memory to sift through a tree, and second to allow for efficient in-memory caching of just the frequently accessed data.

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/benchmark.svg')} />

While Banyan is not yet fully optimized and has plenty of room for improvement, we have already seen impressive benchmark results.
Across a variety of real-world use-cases from existing customers, we achieved an improvement of _4–10x_ for reading events from Banyan compared to ActyxOS version 1.
Yes, you read that right.

**10. Times. Faster.**

<img src={useBaseUrl('images/blog/2.0-release/lightspeed.gif')} />

---

:::info Want to know more about Banyan?
Be sure to check our blog in the coming days as we will soon release a blog post about the Banyan implementation that dives into all its amazing, nitty-gritty, technical details.
:::

## More ergonomic and powerful APIs

Actyx 2.0 comes with a new set of APIs: for one, you can now use the tagging system you already know from Actyx Pond v2 with all event service endpoints.
And secondly we are laying the foundation for inter-app security.
But let’s go over the changes one by one.

### Auth API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/auth-api.svg')} />

The Auth API is a new API that is responsible for checking whether applications are authorized to access the Actyx APIs.
To do that, you need to provide a valid app manifest to the auth API and you will receive a token to be used with HTTP requests to the events API.

This allows us to recognize who is emitting events and tag them accordingly with their app ID.
In the future we will offer more APIs and settings to customize how events of different apps are handled within Actyx.
With these features you’ll be able to protect your IP, configure security boundaries, and more.

### Events API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/events-api.svg')} />

The Events API was formerly known as the Event Service.
It is the core of Actyx and offers users the ability to publish, query, and subscribe to events.
The major update for the Events API is support for tag-based queries instead of the old _name and semantics_ model.
Tags are a highly composable and dynamic approach to distributed event streams.

By switching to a query language we open the door to many more exciting features in the future, like performing filtering or aggregations directly within Actyx — stay tuned!

### Node API

<img style={{borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/node-api.svg')} />

The new Node API allows you to query nodes for information about themselves.
Currently, the API returns the ID of the node.
In the future, more endpoints will be added that will give more insights into the state and performance of a node, so remember to check our [public roadmap](https://trello.com/b/thhTs62O/actyx-product-roadmap)!

Meanwhile, check out the new `ax nodes inspect` command in the Actyx CLI tool.
And don’t forget to [generate a key](/docs/reference/cli/users/keygen) — our node administration access is secured in roughly the same way as an SSH server now.

## A more useful and usable Actyx Node Manager

<img style={{boxShadow: "0 20px 38px -8px rgba(80, 80, 100, .5)", marginBottom: "42px", borderRadius: "6px", border: "1px solid #c5cbd3"}} src={useBaseUrl('images/blog/2.0-release/node-manager-ui.png')} />

The Actyx Node Manager is a graphical desktop application that lets you securely manage and configure decentralized swarms of Actyx nodes in the local network.
With Actyx 2.0 we ship a completely redesigned version of the Node Manager, with better performance and a lot of UX improvements that will significantly streamline working with it.
For instance, Actyx Node Manager now lets you work on multiple nodes at the same time with favorite nodes being saved across sessions so that you can start right where you left off.
For more information on the new Node Manager, please refer to the [reference documentation](/docs/reference/node-manager/) or the release [changelog](/releases).

## Actyx Insider Program

Apart from new features, we are also very excited to announce the **Actyx Insider Program**!

The Actyx Insider Program is a community of Actyx's biggest fans who get to be the first to see what's next.
Actyx Insiders run previews of the platform, then give feedback and engage directly with our Engineers and Product Managers to help shape the future of our product.
Be the first to see what's next for Actyx and join the community and give us your feedback to help make Actyx even better, together.

## Getting Started

For the last months, we worked incredibly hard on this release and we couldn't be more excited to share it with you.
Most of the new features introduced with Actyx 2.0 are a foundation.
A foundation which we will continue to improve and on which we will continue to build.
They unlock a [roadmap](https://trello.com/b/thhTs62O/actyx-product-roadmap) of new features that we are already busy working on.
We have several improvements in progress, such as the Actyx SDK for C# or configuration of ephemeral event streams so be sure to check our [Community Forum](https://community.actyx.com/), [Twitter](https://twitter.com/actyx), or [blog](https://developer.actyx.com/blog) so you don't miss any updates.

Now it is your turn to take Actyx 2.0 for a spin!

To get started, simply [download Actyx](https://developer.actyx.com/releases) and start with one of our [tutorials](https://developer.actyx.com/docs/tutorials/overview).
For a more detailed introduction to Actyx, please check out our [Actyx Academy](https://academy.actyx.com/)!

As always, we are keen on hearing your opinions and feedback.
If you have any questions or requests please visit our [Community Forum](https://community.actyx.com/)!

Happy hacking!
