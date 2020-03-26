---
id: pond-faqs
title: Pond FAQs
permalink: pond/docs/pond-faqs.html
prev: integrating-a-ui
---

Actyx Pond frequently asked questions

### Do I always need onCommand? It seems like a lot of boilerplate

In many "data collection" cases you do not do anything interesting in onCommand. You just convert Command into an event without further checks. Seems tiresome. So you may consider using the following code to just directly emit Events:

```typescript
type Command = Event
const onCommand = (_state, command) => [command]
```

### Fish States differ between nodes even though they are fully in sync! How can this be?

Several possible reasons:

1. If any of the Fish’s subscriptions are local (specify sourceId), the consumed Events will depend on the node running the Fish, hence the resulting State might also differ.
  
2. If `onEvent` is mutating its input parameters, this will most certainly diverge State between nodes. The Actyx Pond relies on these input parameters not being mutated by user code! Always create (deep!) copies, if you need to. If there are performance problems with that, consider using a a library like `immutable-js` which offers high-performance immutable data structures with a great API for creating mutated copies.
  
3. If `onEvent` is reading outside program state, or running non-deterministic logic (like an unseeded Random Number Generator), the result will also differ from node to node. A Fish is run on every node independently, there is no "one single Fish" running across all nodes.
  
4. Your Local Snapshot Serialization Logic may have a bug, or you forgot to increase the version
   number when you changed something about the Fish’s logic.

### I accidentally emitted bad Events. Can I get rid of them?

Any Event persisted on ActyxOS is forever, and immutable.

If you have bad data in some Events, you need to modify the consuming onEvent functions, enabling them to spot the wrongness and act accordingly.

If the bad data was not caused by a programming error, but by operator error, i.e. you expect bad data to appear now and then, you need to enhance your business logic with some notion of "reverse" Events. For example, if your application allows logging consumed input material, but occasionally too much material is logged, you would add functionality to the application that allows flagging mistaken material loggings and emitting this as Events. Then your aggregator can revert information received from earlier Events, once it sees the reverting one.

### My Fish wants to enter an impossible State. How did this happen? What can I do?

Say you are trying to prevent in every way possible to emit a certain Event "Foo" while your Fish has State "Bar". But then in a system with several nodes, it suddenly happens that in onEvent you see State=Bar and Event=Foo. How could that happen?

The answer is that in a partition tolerant distributed system like ActyxOS, nodes still work with incomplete knowledge of "the world." Every node’s knowledge is always out of date by a time at least equal to the network latency towards its peers.

So it may happen that in your conception of the actual reality, State=Bar at some point, but some nodes are not aware of this, and hence do not prevent the emission of Event Foo.

The ideal way to deal with this is to let go of the notion of a real, coherent world. The node that sent the Foo Event was communicating an actual truth from its point of view: Foo happened. Your application should not outright ignore this. Depending on the nature of the thing modeled, there may be several possible responses:

If a thing is impossible, e.g. more material consumed than was present in the batch: This may be an operator error of forgetting to register that a new batch was started. Consider making it visible and letting users amend/resolve the situation manually.

If a thing is very possible but undesired by the process, e.g. goods produced after the order was marked as Completed: Consider that information about the completion hadn’t reached the node yet, so its operators did not know they should stop. They kept producing. The excess output is real and must be made visible and handled.

### What are Commands useful for, exactly?

There is one central, important guarantee about Commands: Every Command handled by a Fish (that is, being passed into onCommand) is guaranteed to see the effects of all previous Commands sent to this Fish on this node.

That is, you can check whether some automated logic in your program was triggered twice: If the Command was already handled, you can deduce so from the State passed into onCommand; then you can decide against emitting the resulting Events another time.

This is important especially in regards to actions taken based on the State of Fishes. Since a Fish may time-travel to incorporate new knowledge, any logic working on State observations – be it via onStateChange, or just pond.observe – may be called repeatedly with States that have identical "maximum Event time," but increasingly more Events from the Past incorporated.

### What is the difference between Commands and Events?

Events are persistent and distributed. An Event, once emitted, will forever be available within your ActyxOS installation.

Commands, on the other hand, are ephemeral and local. If a command is dispatched, it is handled on the node it was created, according to its current knowledge. It is only handled once and forgotten afterward.

### Why are timestamps not always increasing with later events?

ActyxOS uses a technique called "lamport clocks" to sort Events. The important benefit of this is that causal order between Events will always be preserved – even when wall clocks of nodes are not in sync! E.g. if a device’s clock is accidentally set 1 year in the past, its Events will still find their proper place between the other nodes’. But this means that sometimes wall clock time on Events is not increasing with the order of application.

### Why is my Fish not seeing any Events?

Check your subscriptions, if not subscriptions are specified at all, a Fish will listen to its own, **local** Event Stream. That is, it will not see Events from Fishes that live on other nodes, or have different names.

If you want any individual Fish to be distributed, e.g. listen to its "own" Events, but from all nodes:

```typescript
const initialState: InitialState<PrivateState> = (fishName: string) => ({
  state: createInitialState(fishName),
  subscriptions: [
    // By leaving out the third parameter, you make it a distributed subscription!
    Subscription.of(YourFishType, fishName),
  ],
})
```

If you want every individual Fish of your Fish Type to receive Events from _all other Fish of this
Type_, leave out the name as well:

```typescript
subscriptions: [
  // By leaving out the name and the source, we subscribe to all Events from all Fish of your type!
  Subscription.of(YourFishType),
]
```

Please also note that, if any subscription at all is specified, the default local self-subscription is dropped. E.g.

```typescript
subscriptions: [
  // Not at all listening to Events from YourFishType anymore!
  Subscription.of(SomeOtherFishType),
]
```
