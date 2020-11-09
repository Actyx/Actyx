---
title: Introduction
hide_table_of_contents: true
---

Writing distributed apps is difficult.
The **Actyx Pond** framework makes it simpler by providing an opinionated set of tools on top of [ActyxOS](../os/general/introduction).

The idea behind Actyx Pond is that you take your app’s business logic and split it according to responsibilities.
These usually match the physical objects or abstract concepts that your app models and interacts with, be that temperature sensors, robots, receipts, or text messages.
Each responsibility is packaged as a unit called *a fish* and all these fishes swim together in *the pond*, exchanging messages and knowledge by emitting and consuming event streams.

You can imagine Actyx Pond as a scaffolding into which you plug your business logic; your logic makes sense of events and aggregates knowledge in its state, while the pond takes care to keep your business logic up to date with the latest information available.
It is a framework for consuming the event streams provided by the ActyxOS event-focused database, and it helps you make best use of the fully distributed architecture of ActyxOS — it simplifies the creation of *Edge Native* applications.

An application can only fully exploit always-on availability if it acknowledges the constraints that come with this choice.
Your algorithms will make decisions based only on the currently and locally available information: calling a transactional database to achieve strong consistency would prevent the app from functioning while that database is unavailable for any reason.
Actyx Pond therefore makes these constraints explicit and gives you tools for detecting erroneous decisions (caused by incomplete information) and correcting them later.
