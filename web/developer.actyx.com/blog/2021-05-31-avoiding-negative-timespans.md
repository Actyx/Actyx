---
title: How to avoid negative time spans
author: Dr. Roland Kuhn
author_title: CTO and co-founder at Actyx
author_url: https://rolandkuhn.com
author_image_url: /images/blog/roland-kuhn.jpg
tags:
- Local-First Cooperation
- Eventual Consistency
- business intelligence
---

The main advantage of an event-based system like Actyx is that you have a lot of flexibility on how to make even more use of your data when adding new use-cases.
This is achieved by combining the event streams of existing apps with new apps and new business logic.
One issue that frequently arises in these scenarios is that further use, for example in business intelligence tools, exposes a phenomenon that we simply call “negative time spans”.

In this article I explain all about the how and why — the main takeaway is that you should make _really_ sure to use NTP on all nodes.

<!-- truncate -->

One of Actyx’s greatest strengths is that it feels almost magic in how event streams are replicated and state is synchronized between nodes.
The flip side of this coin is that you may be tricked into thinking that it is actually magic, which of course isn’t true.
Under the covers, Actyx just communicates between nodes like me and you would communicate in real life, we don’t use tachyons or subspace relays or any other Star Trek technology 🖖.

## Intuitive ordering within an app

Picture a workflow that spans multiple parties, like the human maintenance worker says “maintenance mode requested for this machine” and the machine then subsequently says “yep, I’m now in maintenance mode”.
The events written in such a flow have a causal relationship, the second one can only have been emitted _because_ the machine gateway has seen and processed the first event.
Therefore, these events will always be ordered correctly by the Actyx system.

:::note
You can still get a negative duration if you subtract the first event’s timestamp from the second event’s timestamp:
if the clock of the first node is ahead or the second node’s clock is lagging, then the timestamps may order differently than the events.
:::

While the above immediately and easily applies to linear workflows, there are some things to watch out for when your workflow [contains cycles](/docs/how-to/actyx-pond/in-depth/cycling-states).
A typical case would be that a production step can be started and stopped multiple times before being finished.
In such cases, it is best to tag each cycle with a unique ID (like a UUID) so that each stop event can be associated with the corresponding start event — otherwise the interpretation of the recorded event history may get confused when multiple start–stop cycles happened concurrently.

## Intuitive ordering between different apps

When events are combined that do not stem from the same logical workflow, things get a bit trickier.
Imagine a machine that does its job, with an app logging performance data and interruptions as Actyx events for display on dashboards or later analysis.
Now imagine a worker using an app for production data acquisition, starting and finishing production steps and logging scrap as well as good pieces;
these data are used for production management and booked into the ERP.

Each of these apps makes sense on its own, they may be programmed by different software vendors.
Since events from one app never depend on events from the other, there are no causal relationships in play here.
Actyx may therefore order these events in some arbitrary fashion, interleaving the streams from both apps.

If you then want to subscribe to both of these event streams in order to extract even more valuable data (like the frequency of interruptions depending on which article is being produced), you need to associate the events and put them into each others’ contexts.
For example you may want to get all machine interruption events between a pair of start & finish events from the worker’s production data acquisition.

The [logical clock](https://en.wikipedia.org/wiki/Lamport_timestamp) used by Actyx to maintain the intuitive ordering between events of the same workflow does not help in this case.
In fact, during network partitions — even brief ones — the logical clock may tick at very different speed on both Actyx nodes, leading to a very much skewed interleaving of the event streams:
the node where less things happen will have its emitted events sorted mostly at the beginning of the partition even when they actually happened near the end.

Therefore, the most appropriate sorting criterion for events between different apps or workflows is to use wall clock time.
You should use that whenever you compute time differences between events, for example in exporters towards ERP or BI systems.
We’ll explain how to do this with the new event functions introduced with Pond version 2.5 in an upcoming blog article (you basically need to keep an array of events sorted by timestamp in your app).

Unfortunately, edge devices don’t have access to a universal and always working wall clock — they cannot walk a few steps to look down the aisle towards the wall of the factory hall like a human would.
And Actyx can also not solve this issue in some magical fashion, in the end the edge device’s clock needs to be set correctly.
You will also want to ensure this if you want to make sense of error or debug logs later;
wrong timestamps are a frequent source of wild goose hunts, wasting a lot of time during a support case.

:::note
So also in this case we need to avoid clock skew, like in the previous section.
:::

## Getting useful timestamps

In both of the above cases the conclusion was that the edge devices do need to have their clocks set correctly.
Actyx is merely a program running on these devices, so we cannot solve this problem in a generic fashion.

:::tip Use NTP!
There is an industry standard for solving this issue: the Network Time Protocol was already created in [1985](https://datatracker.ietf.org/doc/html/rfc958) and is supported by all modern computers.
:::

Besides ensuring proper configuration of NTP on each device, we recommend to also monitor the time on all devices when operating a deployment in a factory.
The reason for this is that due to configuration errors, user intervention, or other problems the device clock may be set incorrectly, leading to lots of headaches later.
With our own customers we observed some cases where a clock jumped by multiple days (backwards or forwards) without an obvious explanation.

## Future directions

Our API currently only supports the so-called Lamport ordering, which as explained above guarantees that events that have a causal relationship are ordered correctly — independent of device clocks.
We will later look into offering a mode where causal relationships are always honored while causally unrelated events are ordered by device timestamp.
This should yield the best possible experience for programmers as well as end users, but it will have a price in terms of event storage size and network bandwidth usage.
But even this mode requires that the device clock be set correctly, so that the ordering of causally unrelated events is in line with what happened in the real world.
