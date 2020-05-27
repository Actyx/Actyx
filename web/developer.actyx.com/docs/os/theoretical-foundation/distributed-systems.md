---
title: Distributed Systems
---

What are disributed system and why are they relevant with ActyxOS?

## Definition

A _distributed system_ is a system composed of multiple software components, running on different networked computers, that communicate and collaborate by passing messages to each other.

![](/images/images/distributed-system.png)

- **Node**: an individual computer, and the software components running on it, of a distributed system
- **Network**: the underlying data communication technology and protocols over which nodes communication
- **Message**: a piece of information that one node sends to one or more other nodes

For a more formal definition and introduction, check out [this paper](https://link.springer.com/article/10.1007/s00607-016-0508-7).

## Benefits

Distributed systems have several benefits that make them very useful in some scenarios.

### Scalability

In a distributed system, computation on one node happens independently from computation on another node. This makes it easy to scale performance and add functionality. In both cases, you can scale the system by adding additional nodes that either duplicate existing or implement new functionality.

### Reliability

Because computations are split over multiple nodes&mdash;vs. a single computer&mdash;distributed systems are significantly more reliable. When faults occur (it happens), only the functionality provided by the affected nodes is unavailable. Other nodes can continue working; there are no single-points-of-failure that can take down the entire system.

### Performance

Distributed systems allow you to split your workload into multiple tasks and have nodes work on them in parallel. This allows you to increase the overall performance of your system. Additionally, you benefit from improved communication performance as individual groups of nodes communicate over the sub-network, without affecting communication in other parts of the network.

## Challenges

Distributed systems pose a number of challenges to system architects and developers building software for them.

### Coordination

Because computation is split amongst multiple nodes, additional effort needs to go into coordinating who does what and when. This is especially challenging because individual nodes may be disconnected or in a failure-state at any time. As a coordinator, you need to be able to deal with all these situations.

### Latency volatility

With increased communication&mdash;some of it a pure overhead&mdash;and very dynamic system behavior, communication between nodes may at times be very fast, and at other times slow. This happens, for instance, when nodes coordinate complex tasks or synchronize amongst each other following downtime.

### Monitoring

The behavior of a distributed system is significantly harder to reason about than a centralized, single-node system. Nodes may be up or down at any time, communication may be faster or slower, etc. This make monitoring a distributed system significanly harder than a centralized system.

## Distributed vs. decentralized

A distributed system may be coordinated by a central node. Consider, for example, how map-reduce jobs are usually launched by a central node, the results computed in a distributed fashion, and the results reduced back into the central node.

_Decentralized_ systems are a special form of a distributed system where there is no such central coordination authority. _ActyxOS_ is a distributed, but especially a completely decentralized system, with no central authority that distributes and coordinates work.

## Relevance to ActyxOS

By building and running apps on ActyxOS you are building a _decentralized_, and thus also a _distributed_, system. Edge devices provide the computing power and access to the underlying network. Apps run on nodes and communicate by passing messages (see [Event Streams](../api/event-service.md)). Blobs can be published by any node and accessed from any other node (see [Blob Storage](../api/blob-service.md)).

With ActyxOS you get all the benefits of distributed systems: **scalability**, **reliability**, and **performance**. ActyxOS also tries to reduce the challenges associated with a distributed system to a minimum. If you build apps using the [Actyx Pond](../../pond/introduction)&mdash;a framework for building always-available apps&mdash;many of the challenges are completely taken care of for you.

## Learn more

Here are several resources that you can check out to dive deeper into distributed systems:

- [Distributed systems: A quick and simple definition (O'Reilly)](https://www.oreilly.com/ideas/distributed-systems-a-quick-and-simple-definition)
- [A Thorough Introduction to Distributed Systems](https://www.freecodecamp.org/news/a-thorough-introduction-to-distributed-systems-3b91562c9b3c/)
- [Designing Distributed Systems (O'Reilly, E-Book)](https://azure.microsoft.com/en-us/resources/designing-distributed-systems/)