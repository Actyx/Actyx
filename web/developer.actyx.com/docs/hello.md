---
id: hello
title: Installation Guide
hide_title: false
hide_table_of_contents: false
sidebar_label: Placeholder Sidebar Title
keywords: [keyword1, keyword2]
description: Placeholder page to test things out.
image: /images/defaults/default.svg
---

## Introduction to the Actyx Platform

> What is this looking like?
> This is a quote i believe

<!-- truncate -->

The Actyx platform is an application platform for developing, deploying, running and monitoring local-first applications, such as factory automation on the shop-floor where utmost reliability is required.
With the Actyx platform we bring a modern development experience to this space and provide you with the software and tools that enable you to build better solutions faster.

:::info
And this is a callout
:::

On the Actyx platform, your applications run directly on the edge without the need for a centralized IT infrastructure.
Every part of the process that you want to digitally connect — be it a machine, a robot, a worker or an ERP system — receives an edge device.
The business logic runs directly on these edge devices and communication happens in a peer-to-peer fashion.
This architectural decision enables us to provide a set of non-functional guarantees that are hard to achieve in an even partly centralized system.

```typescript
import { Client, EventDraft } from '@actyx/os-sdk'

const ActyxOS = Client()

// callback style
ActyxOS.eventService.publish({
  eventDrafts: EventDraft.make('mySemantics', 'myName', { foo: 'bar' }),
  onDone: () => {
    console.log(`Published`)
  },
})

// promise style
await ActyxOS.eventService.publishPromise({
  eventDrafts: EventDraft.make('mySemantics', 'myName', { foo: 'bar' }),
})
```

Solutions that run on the Actyx platform can stay operational in the face of component failures and even when parts of the system are disconnected — down to individual devices — and can keep making progress.
In a setting like a factory or hospital with extremely high uptime requirements such resilience is a significant advantage.

## Another second level headline

With the Actyx platform we also support you along the entire application lifecycle.
On the Actyx platform you develop your applications using the software abstractions that our distributed middleware provides without needing to worry about setting up databases or message buses.
Our suite of developer tools supports you with packaging and deploying your applications directly to edge devices.

### Maybe a third level headline

On the edge devices your applications are reliably run in the application runtimes that we provide and can be monitored without needing to set up any additional tools.

And this is just the beginning.
As we continue developing the Actyx platform we will take more and more things off your shoulders.
From providing ready made application components to improving the tools that you use to develop, deploy and monitor your applications we work hard to move the development experience forward.

Please also feel free to check out [this video](https://www.youtube.com/watch?v=T36Gsae9woo) which provides a more visual introduction into some of the core features of the Actyx platform.
