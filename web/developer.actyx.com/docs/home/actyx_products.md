---
title: Actyx Platform Products
sidebar_label: Actyx Platform Products
hide_title: true
hide_table_of_contents: true
---

## Actyx Platform Products

![](/images/home/actyx_products.png)

The Actyx platform itself comprises several different products and tools.
Here we provide an overview of some of the most important ones.

### ActyxOS

At the core of the Actyx platform is a piece of software that we call ActyxOS.
Every device in your setup that you want to connect either runs the ActyxOS application or is connected to another device that runs it; Currently supported operating systems are Windows, Linux and macOS.
ActyxOS provides the application runtimes to easily deploy and run your application logic on the edge devices, an event service abstraction for seamless communication between different devices and additional middleware services around storage and deployment management.

It is possible to write your application logic directly on the middleware abstractions that ActyxOS provides by using the ActyxOS [APIs](os/api/event-service.md) or language specific [SDKs](os/sdks/js-ts.md).
However, we usually recommend to start writing your solutions with Actyx Pond which builds on top of ActyxOS.

To learn more about the specific services that ActyxOS provides and how you can start using it please start [here](os/introduction.md).

### Actyx Pond

Actyx Pond is an opinionated TypeScript framework that is built on top of the middleware abstractions that ActyxOS provides.
At the core of Actyx Pond lies a powerful programming model that makes it easier for you to write correct event-driven applications.
This programming model can take some time getting used to but once you get the hang of it provides you with a well structured approach to build sophisticated solutions for your use-cases.

To learn more about the Actyx Pond programming model and how you can start writing solutions on top of Actyx Pond please start [here](pond/getting-started.md).

### Actyx CLI

The Actyx Command Line Interface (CLI) is a unified tool for managing your ActyxOS nodes and apps.
The Actyx CLI enables you to easily deploy, configure and monitor your ActyxOS environment.

You can find more documentation on the capabilities of the Actyx CLI [here](cli/getting-started.md).