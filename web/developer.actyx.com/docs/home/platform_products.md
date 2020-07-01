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
Every device in your setup that you want to connect either runs ActyxOS or is connected to a device that runs ActyxOS.
ActyxOS provides both the application runtimes to easily deploy and run your application logic on the edge devices and the middleware services that are needed to build holistic process automation solutions across different devices.

It is possible to write your application logic directly on the middleware abstractions that ActyxOS provides by using the ActyxOS [APIs](os/api/event-service.md) or language specific [SDKs](os/sdks/js-ts.md).
However, we usually recommend to start writing your solutions with Actyx Pond which builds on top of ActyxOS.

To learn more about the specific services that ActyxOS provides and how you can start using it please start [here](os/introduction.md).

### Actyx Pond

Actyx Pond is an opinionated TypeScript framework that is built on top of the middleware abstractions that ActyxOS provides.
At the core of Actyx Pond lies a powerful programming model that makes it easier for you to write correct event-driven applications.
This programming model can take some time getting used to but once you get the hang of it provides you with a well structured approach to build sophisticated solutions for your use-cases.

To learn more about the Actyx Pond programming model and how you can start writing solutions on top of Actyx Pond please start [here](pond/getting-started.md).

### Actyx CLI

A more sophisticated use-case will involve running many instances of ActyxOS and applications that build on top of it across your different devices.
To make it easier to deploy, configure and monitor these setups we provide you with the Actyx Command Line Interface (Actyx CLI) tool.

You can find more documentation on the capabilities of the Actyx CLI [here](cli/getting-started.md).