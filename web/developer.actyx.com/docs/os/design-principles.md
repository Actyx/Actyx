---
title: Design Principles
---

- **Availability over consistency** - We favor availability of each individual node to consistency of the system as a whole. Your app should always work, even when other nodes are unavailable

- **Deterministic, orthogonal APIs** - APIs should have well defined behaviors and guarantees you can rely on and should be orthogonal; we like tools that do one thing but do that thing well

- **Layered architecture** - We want to seperate concerns between layers. That is why ActyxOS is composed of different services and is completely independent of the [Actyx Pond](../pond/introduction.md)

- **Sensible defaults** - Where possible we provide sensible defaults to flatten the learning curve. If you have specific needs, please [reach out](introduction.md#contact-us--something-missing) and we can help you tune ActyxOS to your needs
