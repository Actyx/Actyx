---
title: How do I not run out of disk space?
sidebar_label: Running out of disk space
---

Use retention-policies for blobs. More coming soon...

ActyxOS is a completely decentralized system dependent on the disk space of every single edge device. At runtime different types of data are generated and stored throughout the ActyxOS swarm.

| Type    | Size     | Deletable?                                                                            |
|---------|----------|---------------------------------------------------------------------------------------|
| Event   | max. 4KB | **No**, events have an infinite lifespan unless you reset the swarm.                  |
| Blob    | max. 4GB | Yes, using retention policies and the automatic ActyxOS garbage collector.            |
| Log     | max. 4KB | Yes, the AcytxOS Console Service clears logs automatically when disk space is needed. |

Given correct usage of retention policies, blobs should never lead to running out of disk space. Log should not cause this either. Only a large number of events can lead to disk space issues.

The point at which you will run out of disk space because of events depends on their number and size. 100 million events with an average size of 0.2KB will require a maximum of 20 GB of disk space.

Currently, the only solution to running out of disk space because of events, is either clearing events from your swarm or increasing the disk space of your edge devices.

:::info Heuristic distribution is on our roadmap
We are working on heuristic distribution, whereby not all events are distributed to everyone. This will seriously reduce the amount of necessary disk space. Stay tuned to our [blog](https://www.actyx.com/news) for updates regarding this.
:::
