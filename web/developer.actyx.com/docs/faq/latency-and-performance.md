---
title: ActyxOS latency and performance
sidebar_label: Latency and performance
---

No guarantees possible, but here are some general characteristics.

Runtime performance and communication latencies are extremely hard to predict. Hundreds of things play into this, from the hardware of your edge devices to a forklift passing in front of a Wireless Access Point.

In terms of runtimes, the following statement are&mdash;on average&mdash;true:

- The ActyxOS [WebView Runtime](../os/advanced-guides/app-runtimes.md#webview-runtime) can run any application with good end-user performance
- The ActyxOS [Docker Runtime](../os/advanced-guides/app-runtimes.md#docker-runtime) can run I/O-heavy applications with good performance
- The ActyxOS runtimes are not deterministic in their performance (use a PLC if this is needed)

In terms of communication latency, the following statements are&mdash;on average&mdash;true:

- ActyxOS can conistently achieve sub-second latency of approx. 200ms
- ActyxOS has lower latency volatility than most centralized systems
- ActyxOS is not deterministic in terms of latency (use a PLC if this is needed)

:::warning Disclaimer
As a developer you can always build apps that will bring any system to its knees. If you follow best-practices you should not face any issues. If you do, please [get in touch](https://www.actyx.com/contact)&mdash;we love optimizing!
:::