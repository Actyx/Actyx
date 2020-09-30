---
title: onEvent
hide_table_of_contents: true
---

`onEvent` is the function used to aggregate _events_ into _state_. Conceptionally it is very similar
to the function you pass to
[Array.reduce](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/reduce). Think
of the sorted event log as the array and the state as the accumulation object.

## Time Travel and Keeping `onEvent` Pure

The most important thing to keep in mind when implementing `onEvent` is that it may be called again
and again with very similar inputs. This is due to time travel. When an event _from the past_
arrives, it is inserted into its proper spot in the log of all relevant events. Then, in order to
compute an updated correct state, the `onEvent` aggregation is run again over the complete event
log.

[Read a concrete example in our tutorial.]()

Due to this, it is very important that `onEvent` is a "pure function." A pure function is a function
where the output depends _only_ in the inputs.

The following are examples of things that are NOT pure and hence must not be done inside
`onEvent`:

- Looking at the current time via `new Date()` or similar. If you need to get the time at which an
  event occured, look at the `metadata`.
  
- Accessing dynamic global state.

- Modifying anything that is not part of the output state. 




## The Inputs

- `state: S` – the current state of the Fish, i.e. a state to which all _previous_ events have
  already been applied
  
- `event: E` – the current event to apply

- `metadata: Metadata` – a collection of various metadata tied to the event. See ....

## The Output

The function must return a value of type `S`. The following are all legal:

- Returning the input state, unchanged. Although note that you [should not ignore events]

- Modifying the input state and returning it.

- Returning a completely new object.

The returned value will then be fed as input `state` to the `onEvent` invocation for the next
event.

It will also be potentially published to observers that have called `pond.observe` for this
Fish. ----



