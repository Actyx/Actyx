---
title: Exception Handling
---

It is important to write the functions contained in a Fish – `onEvent`, `deserializeState` and `isReset` – very defensively.
If they throw errors, there is often no reasonable way to continue running a Fish; because its final state depends on all previous states.

:::warn
A Fish is **stopped** when one of its function throws an error.

It’s most recommended you wrap your complete `onEvent` inside a try/catch block and implement exception handling that makes sense for that specific Fish.
:::

## Fish-Specific Error Callbacks

When invoking `pond.observe()` or `pond.observeOne()`, you can optionally pass a `stoppedBy` callback as final argument.

If one of the Fish’s functions throws an error, it will be passed to `stoppedBy`.

Even if you do not supply the callback, every Fish will be stopped after throwing an error.

## General Error Reporting

The options passed when creating a Pond can define a `fishErrorReporter`. For example:

```ts
const pond = await Pond.default({
  fishErrorReporter: (err: unknown, fishId: FishId, context: FishErrorContext) =>
      console.error('Error while executing', FishId.canonical(fishId), ':', err, context)
})
```

By default, the information is printed to `console.error` like shown in the example.

You can define your own `fishErrorReporter` in order to get the errors into the logging framework of your choice,
push them to an error tracking service, or whatever else you like.

Note that defining your own `fishErrorReporter` disables the default behavior.
If you define a custom `fishErrorReporter` function and want to keep the outputs to `console.error`,
you have to make that a part of your `fishErrorReporter` function.
