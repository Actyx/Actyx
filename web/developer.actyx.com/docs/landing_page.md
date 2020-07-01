--- 
title: Introduction to the Actyx Platform
---

## The Actyx Platform

![](/images/landing_page/actyx_platform.png)

The Actyx platform is an application platform for developing, deploying, running and monitoring factory process automation solutions. With the Actyx platform we bring a modern development experience to the factory automation space and provide you with the software and tools that enable you to build better solutions faster.

On the Actyx platform your solutions run directly at the edge without the need for a centralized IT infrastructure. Every component in your process that you want to connect — be it a machine, a robot, a worker or an ERP system — receives an edge computer. The logic runs directly on these edge computers and communication happens in a peer-to-peer fashion. This architectural decision enables us to provide a set of non-functional guarantees that are hard to achieve in a more centralized system; Solutions that run on the Actyx platform can stay operational in the face of component failures and even in the face of network partitions the different components can keep making progress. In a setting like a factory with extremely high uptime requirements such resilience can be a huge advantage. 

But the Actyx platform is not just useful due to its strong non-functional guarantees. With the Actyx platform we also support you along the entire application lifecycle. On the Actyx platform you develop your applications using the software abstractions that our distributed middleware provides without needing to worry about setting up databases or message buses. Our suite of developer tools supports you with packaging and deploying your applications directly to edge devices. On the edge devices your applications are reliably run in the application runtimes that we provide and can be monitored without needing to set up any additional tools. And this is just the beginning. As we continue developing the Actyx platform we aim to take more and more things off your shoulders. From providing ready made application components to improving the tools that you use to develop, deploy and monitor your applications we work hard to move the development experience forward.

Please also feel free to check out [this video](https://www.youtube.com/watch?v=T36Gsae9woo) which provides a more visual introduction into some of the core abstractions of the Actyx platform.

## Actyx Platform Products

The Actyx platform itself is made up of several different products and tools. Here we provide an overview of some of the most important ones.

### ActyxOS
At the core of the Actyx platform is a piece of software that we call ActyxOS. Every device in your setup that you want to connect either runs ActyxOS or is connected to a device that runs ActyxOS. ActyxOS provides both the application runtimes to easily deploy and run your application logic on the edge devices and the middleware services that are needed to build holistic process automation solutions across different devices.

It is possible to write your application logic directly on the middleware abstractions that ActyxOS provides by using the ActyxOS [APIs](os/api/event-service.md) or language specific [SDKs](os/sdks/js-ts.md). However, we usually recommend to start writing your solutions with Actyx Pond which builds on top of ActyxOS.

To learn more about the specific services that ActyxOS provides and how you can start using it please start [here](os/introduction.md).

### Actyx Pond
Actyx Pond is an opinionated TypeScript framework that is built on top of the middleware abstractions that ActyxOS provides. At the core of Actyx Pond lies a powerful programming model that makes it easier for you to write correct event-driven applications. This programming model can take some time getting used to but once you get the hang of it provides you with a well structured approach to build sophisticated solutions for your use-cases.

To learn more about the Actyx Pond programming model and how you can start writing solutions on top of Actyx Pond please start [here](pond/getting-started.md).

### Actyx CLI
A more sophisticated use-case will involve running many instances of ActyxOS and applications that build on top of it across your different devices. To make it easier to deploy, configure and monitor these setups we provide you with the Actyx Command Line Interface (Actyx CLI) tool.

You can find more documentation on the capabilities of the Actyx CLI [here](cli/getting-started.md).

## Getting Started
The fastest way to see the Actyx platform in action is to check out our [quickstart guide](quickstart.md).

We also provide a half-day ramp up session with one of our engineers to get you started developing on the Actyx platform. If you are interested please [contact us](https://www.actyx.com/contact) or join us in our [Discord chat](https://discord.gg/262yJhc) and we will be happy to set you up or answer any questions you might have.