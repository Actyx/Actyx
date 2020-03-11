/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
# Command API

The onCommand method of a fish processes commands and produces events. The type signature of onCommand is

```ts
type OnCommand<S, C, E> = (state: S, command: C) => ReadonlyArray<E> | CommandApi<ReadonlyArray<<E>>
```

# Generating events

To return events, it is possible to return just an array of appropriately typed events.

```ts
const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'enable':
      return state.type === 'initial' ? [{ type: 'enabled' }] : []
    case 'disable':
      return state.type === 'enabled' ? [{ type: 'disabled' }] : []
    case 'ping':
      return state.type === 'enabled' ? [{ type: 'pinged' }] : []
    case 'reset':
      return [{ type: 'reset' }]
    default:
      return unreachableOrElse(command, [])
  }
}
```

To help type inference, it is sometimes necessary to explicitly annotate the return type. This is partially due to the fact that a JS array literal `[]` needs to be coerced to a `ReadonlyArray<E>`. A way to help typescript is to have a simple, properly annotated identity function

```typescript
const events = (...es: Event[]): ReadonlyArray<Event> => es
const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    ...
    case 'finishStep':
      return events({ type: 'workFinished', step: command.step })
    ...
  }
}
```

# Performing effects and generating events

All possible effects that can be produced by the CommandApi are defined in the [CommandApi](/pond/ada/commandApi.ts) companion object. It is possible to interact with http, ipfs and the pond.
Interactions in the CommandApi such as http operations allow handling errors (unlike the ones in OnEvent) by returning an either.

# Chaining effects

Effects are chainable. The final result of a chain must be a `ReadonlyArray<E>` of events of the appropriate type for the fish.

```ts
const slowCommand = (cmd: SlowCommand): AsyncCommandResult<Event> =>
  noEvents
    .chain(() => pond.send(cmd.target)(`starting ${cmd.id}`))
    .chain(() => http.get('http://slow.com'))
    .chain(() => pond.send(cmd.target)(`ending ${cmd.id}`))
```

Effects can also be performed in parallel using the CommandApi.all method. The CommandApi object has monadic properties and tries to conform to the [fantasyland](https://github.com/fantasyland/fantasy-land) specification for algebraic types in javascript.

# Pure side effects

API methods that perform just a side effect, such as `pond.send`, return an empty array of type `ReadonlyArray<never>` for convenience of use. E.g.
```ts
const onCommand: OnCommand<State, Command, Event> = (_, command) => {
  switch (command.type) {
    ...
    case 'send':
      return pond.send(command.target)(command.cmd)
    ...
  }
}
```
returns an empty array of events and can thus be directly used in onCommand

# Logging

Logging can of course be performed as a side effect in transformation stages. In addition, the CommandApi provides a logging function which logs the current value to a log function on execution.

```ts
const ipfsImport: (url: string) => AsyncCommandResult<Event> = (url: string) =>
  http
    .get(url)
    .log()
    .chain(data =>
      liftCE(ipfs.dag.put)(data)
        .log()
        .map(r =>
          r.fold<ReadonlyArray<Event>>(_ => [], hash => [{ type: EventType.Imported, url, hash }]),
        ),
    )
```

# Mixing both

It is possible to mix both styles of defining onCommand handlers. The type will usually be properly inferred as `ReadonlyArray<E> | CommandApi<ReadonlyArray<<E>>`. However, in this case typescript frequently needs a bit of assistance. A simple way to ensure that types are properly inferred is to factor out more complex command handlers as methods. That also simplifies finding errors and improves the overall structure of the code.

```ts
const ipfsImport: (url: string) => AsyncCommandResult<Event> = (url: string) =>
  http
    .get(url)
    .log()
    .chain(data =>
      liftCE(ipfs.dag.put)(data)
        .log()
        .map(r =>
          r.fold<ReadonlyArray<Event>>(_ => [], hash => [{ type: EventType.Imported, url, hash }]),
        ),
    )

const onCommand: OnCommand<State, Command, Event> = (_, command) => {
  switch (command.type) {
    case 'import':
      return ipfsImport(command.url)
    case 'ping':
      return [{ type: 'pinged' }]
  }
}
```

# Dealing with eithers

When working with the command API, the returned value is often wrapped in a either to be able to distinguish between success and failure. So a frequently required type is
```ts
type CE<E, V> = CommandApi<Either<E, V>>
```

To simplify working with such values, there are a number of utility functions that lift simple functions to functions that return a `CE`. They are defined on the companion object of CE.

# Testing

Testing onCommand methods can be done using the [TestCommandExecutor](/pond/ada/testkit/testCommandExecutor.ts). It produces a result that describes both the intended effects and the returned events. The result can be checked either explicitly or using a snapshot test.

A more integrated form of testing is provided by the [FishTestFunctions](/pond/ada/testkit/FishTestFunctions.ts).

# Avoiding command queue growth

During the execution of the effects of a command, the fish will not process other commands. This can effectively block the fish while a long operation such as a http request to a slow or unresponsive endpoint is in progress. Commands arriving at that time will be enqueued and executed in order.

When combined with a state effect pipeline that sends commands to the fish at a regular interval, this can lead to the dangerous effect of sending more commands than the fish is able to handle for an extended time period.

The best way to avoid this is to send a self command once the operation is completed. Other options are sending self commands from the state pipeline at low frequency or triggered by a state change.
