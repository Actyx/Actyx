---
title: Eventual Consistency
hide_table_of_contents: true
---

The Actyx Pond offers easy access to a distributed system that chooses Availability over
Consistency, in terms of the [CAP Theorem](https://en.wikipedia.org/wiki/CAP_Theorem). That means:

- Full Availability: Applications keep working, even if they become disconnected. I.e. when an
  ActyxOS node becomes partitioned in the network, all its apps are still completely usable.

- Potential Loss of Consistency: Between the events that you already see, and the events you emit,
  there may eventually appear _more_ events.

The latter is the logical consequence of the first: The node that is allowed to go on by itself, for
a while, will eventually be connected again. When this happens, all events that were created in the
meantime are exchanged. No events are discarded.

According to a distributed clock mechanism ([Lamport
time](https://en.wikipedia.org/wiki/Lamport_timestamp)), one canonical order for all events is
settled. In this way, as soon as nodes have knowledge of the same set of events, they can also agree
on their _order_. When they agree on their order, aggregation (like onEvent) can run over the
time-line of events, and will yield the same **consistent** result everywhere.

During a network partition, nodes will neccessarily be in disagreement. Once the partition is over,
they will eventually reach agreement.

:::tip
Even well-connected nodes can be thought of as being partitioned by their network latency. There is no such
thing as perfect or instantaneous connectivity.
:::

# Impact on Application Development

It’s important to keep the Eventual Consistency model in mind when designing applications on
ActyxOS. Seemingly contradictory information may be created on different nodes. But the
contradiction is likely just a true image of the real world, where things often fail to go as
intended: For example, some misunderstanding causes two people to start working on the same task,
even though just one of them was supposed to do it. After a couple of confused phone calls, the
situation is finally cleared up.

## When faced with contradictory information, make it visible!

An ActyxOS app can be a huge improvement over confused phone calls, by making contradictions _visible_ and
offering help with resolving botched situations. But the actual resolving, in the real world, must
be left to humans.

Traditional applications often place a lot of restrictions on what can be done in a certain
situation. In ActyxOS apps that is usually a bad idea. In the real world, the damage may already
have been done! The user must be able to make the issue visible.

Hence

- Do not place too many restrictions on your user interface. Warn about unintended usage! But do
  allow it.
  
- [Do Not Ignore Events.](/docs/pond/in-depth/do-not-ignore-events)
  
# Being Aware of Connectivity Issues

<!-- TODO: Dedicated page or something for getNodeConnectivity -->

The `Pond` offers a function called `getNodeConnectivity` which gives information about how well
connected the underlying ActyxOS currently is. It’s a good idea to make this information available
in the UI via some small indicator ([example](https://github.com/Actyx/industrial-ui/tree/master/src/components/NetworkStatusIcon)).

Do note however that connectivity quality can only be measured for the past! Even if the last
network message reached its destination within a millisecond, the next one may already get dropped –
it’s impossible to predict with certainty.

# True Consistency

True consistency can – at the cost of availability – be achieved by using so-called "consensus
algorithms." In theory, ActyxOS’ eventually consistent event system is entirely sufficient for
building consensus algorithms on top of it. However, the implementation is not trivial.

A future release of ActyxOS will ship with a native consensus implementation akin to
[Paxos](https://en.wikipedia.org/wiki/Paxos_(computer_science)). Based on that will be offered
high-level interfaces that hide the underlying complexity of achieving consensus.
