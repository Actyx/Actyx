---
title: Architecture
hide_table_of_contents: true
---

On a technical level, ActyxOS combines two things:

- a novel database dealing in flexible event streams, fully decentralised and thus available locally on each edge device — you could imagine this part as a message broker where the readers create the topics by their event stream queries, but without any central infrastructure
- the ability to host apps and manage their settings and lifecycle

Your apps thus interact with both a runtime environment and event stream services, both offered by ActyxOS:

![ActyxOS Architecture](/images/os/architecture.svg)

ActyxOS provides five core services and, where applicable, associated APIs or deployment tools. These services either provide stand-alone functionality or allow you to build your own apps.

Refer to the following sections to learn more about the different services:

- Learn about the WebView Runtime and the Docker Runtime in the [Running Apps](guides/running-apps.md) section
- Learn about the Event Service in the [Event Streams](guides/event-streams.md) guide
- The Blob Service is still in Alpha, but you can check out it's [preliminary API](/docs/os/api/blob-service)
- Learn how to use the Console Service in the [Logging](/docs/os/api/console-service) and [App Runtimes](advanced-guides/app-runtimes.md) sections
