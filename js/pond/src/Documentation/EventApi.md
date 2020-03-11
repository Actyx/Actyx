/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
# Event API

The onEvent method of a fish processes events and produces a new state. The type signature of onEvent is

```typescript
type OnEvent<S, E> = (state: S, event: Envelope<E>) => S | EventApi<S>
```

# Directly producing a new state

To just produce a new state immediately, just return an appropriately typed new state `S`

```typescript
const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  switch (event.payload.type) {
    case 'enabled':
      return { type: 'enabled' }
    case 'disabled':
      return { type: 'disabled' }
    default:
      return unreachableOrElse(event.payload, state)
  }
}
```

# Performing ipfs operations and generating new state

All possible fundamental ipfs operations that are permitted by the EventApi are defined in the [EventApi](../eventApi.ts) companion object. The number of possible interactions is extremely limited. Currently it is only possible to interact with ipfs in an observably referentially transparent way. E.g. it is not possible to detect a resolution failure of an IPFS object lookup, because that would expose nondeterminism, which is not permissible in onEvent methods.

# Chaining effects

Effects are chainable. The final result of a chain must be a new state `S`.

```typescript
const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  switch (event.payload.type) {
    case EventType.Imported: {
      const { key, cid } = event.payload
      return ipfs.dag.get(cid).map(value => ({
        data: assoc(key, value, state.data),
      }))
    }
  }
}
```

Effects can also be performed in parallel using the EventApi.all method. The EventApi object has monadic properties and tries to conform to the [fantasyland](https://github.com/fantasyland/fantasy-land) specification for algebraic types in javascript.

# Mixing both

It is possible to mix both styles of defining onEvent handlers. The type will usually be properly inferred as `S | EventApi<S>`. However, in this case typescript frequently needs a bit of assistance. A simple way to ensure that types are properly inferred is to factor out more complex command handlers as methods. That also simplifies finding errors and improves the overall structure of the code.

# Testing

Testing onCommand methods can be done using the [TestEventExecutor](../testkit/testEventExecutor.ts). It runs the effects synchronously and directly produces the new state.

A more integrated form of testing is provided by the [FishTestFunctions](../testkit/FishTestFunctions.ts).
