# Why Ada, ActyxOS, Actyx have only events and no commands

_This is a retrospective write-up of a decision made in 2017 by @rkuhn and @rklaehn._

## Decision

Actyx replicates only events between network nodes.
Our middleware only models immutable facts that can freely be shared after being created.
In particular, we do not support sending commands over the network.

## Background

Other systems support two fundamentally different kinds of communication:

- _commands_ convey intent and are ephemeral, they typically also have a finite lifespan due to timeouts
- _events_ record facts that cannot be altered and remain valid for an indefinite timespan

Commands are usually exchanged between clients and the server, whereas the server emits events based on those commands whenever appropriate.
Our Fish abstraction offers the same distinction, with `onCommand` playing the role of the classic server (albeit with less power since it is not running in one central place).

We decided very early to focus on immutable data when designing inter-node communication and information replication.
This makes everything a lot easier since immutable facts can just be copied and shared without having to worry about invalidation and updates.
Aforementioned reason stays relevant even though application logic will often model mutable state — the decision to make the basic abstraction immutable was driven by the assumption that mutability adds essential complexity that is best handled on the app layer since the infrastructure cannot _solve_ any significant part of it.
A proper solution in this sense would be fully managed state, where Actyx provides the available data types with fixed semantics (i.e. the CRDT approach).

Another reason is that we intend our event log to be the system of record on the shop floor, a perfect audit log because it must necessarily record all relevant information.
In a recently documented example (partial connectivity UX blog), a “command” is recorded as an event on a tablet and reacted upon by a machine gateway app.
If we moved the command to a separate channel the machine gateway app would still need to persist it to record who ordered it and when.

The long-term direction of going towards real peer-to-peer message transport (i.e. mesh networks) is one more reason for avoiding commands:
it would be rather difficult to find command delivery guarantees that are easy to understand and intuitively useful to the developers programming on Actyx.
Sending commands only to directly connected peers would be surprising since connection status is managed by Actyx and invisible to the programmer.
Forwarding commands throughout the mesh network would need clearly defined limits, but what should happen when they are exceeded?

Yet another question with no obviously intuitive answer is how commands should be ordered with regards to the delivery of events.
Persisting the intent of needing something to be done via an event has clearly defined at-least-once delivery semantics and ordering relative to actions by the same node or causally dependent actions.

## Consequences

Our system records facts and allows thing at all times on all nodes, which implies that our system cannot be the arbiter in contentious decisions.
The latter would require implementing some form of consensus (e.g. by referee, or a well-known algorithm like Raft).

One further consequence of this is that our system is attuned to scenarios where the network is not reliable and individual nodes can fail at any time.
If the target system is operated such that hardware, software, and network availability are guaranteed, then deploying Actyx leads to extraneous software development costs due to preparing for an eventuality that is covered elsewhere — our hypothesis is that such an environment is not frequently found in factories.
