--- 
title: Introduction
---

## What is the Actyx Platform?

![](/images/landing_page/actyx_platform.png)

The Actyx platform is an application platform that supports you in building solutions to digitize and automate factory processes. At a high level this means that we provide you with a set of binaries, tools and software libraries that make it easier for you to get started building your use-case and spend less time solving distributed systems problems or setting up the infrastructure.

## What does the Actyx Platform provide?

The Actyx platform itself is made up of a number of different products and tools. Here we provide an overview of some of the most important ones.

![](/images/landing_page/actyx_product_overview.png)

Let's go through them one by one so you can gain a better understanding of the kinds of services that the platform provides.

### ActyxOS
At the core of the Actyx platform is a piece of software that we call ActyxOS. Every device in your setup that you want to connect - be it to read out from or send data to a machine, display information on a tablet or connect to an ERP system - either runs ActyxOS or is connected to a device that runs ActyxOS.

ActyxOS provides the plumbing that makes it possible to easily deploy applications to devices and enable these different devices to connect to and communicate with each other. All the important abstractions that you need to build solutions are provided. For example, one core functionality of ActyxOS is that it acts as a persistent event streaming system, somewhat similar to something like Apache Kafka, where different processes on the different devices can publish, subscribe to and query events. However, ActyxOS provides these services without the need for any centralized servers and runs completely peer-to-peer on the edge devices. 

It is possible to write applications directly on the abstractions that ActyxOS provides by using the ActyxOS [APIs](os/api/event-service.md) or [SDKs](os/sdks/js-ts.md). However, we usually recommend to start writing your solutions with Actyx Pond which builds on top of ActyxOS and which we will explore next.

To learn more about the specific services that ActyxOS provides and how you can start using it please start [here](os/introduction.md).

### Actyx Pond
Actyx Pond is an opinionated TypeScript framework that is built on top of the abstractions that ActyxOS provides. At the core of Actyx Pond lies a powerful programming model that makes it easier for you to write correct event-driven applications in a distributed environment. This programming model can take some time getting used to but once you get the hang of it it provides you with a well structured approach to build sophisticated applications for factory use-cases.

To learn more about the Actyx Pond programming model and how you can start writing applications on top of Actyx Pond please start [here](pond/getting-started.md).

### Actyx CLI
A more sophisticated use-case will involve running many instances of ActyxOS and applications that build on top of it across your different devices. To make it easier to deploy, configure and monitor these setups we provide you with the Actyx Command Line Interface (Actyx CLI). It is a unified command line tool that helps you in managing your nodes and apps. For example, with Actyx CLI you can easily deploy applications to specific ActyxOS nodes from your local machine or start, stop and configure the ActyxOS nodes in your setup.

You can find more documentation on the capabilities of the Actyx CLI [here](cli/getting-started.md).

## Getting Started
The fastest way to see the Actyx platform in action is to check out our [quickstart guide](quickstart.md).

We also provide half-day ramp up sessions with one of our engineers to get you started developing on the Actyx platform. If you are interested please [contact us](https://www.actyx.com/contact) and we will be happy to set you up or answer any questions you might have.