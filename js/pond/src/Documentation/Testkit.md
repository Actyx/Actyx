/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
# Testing fishes

All test functions that might be useful for other libraries are in [testkit](../testkit).

## Unit tests

A fish is just a bunch of functions, onCommand, onEvent etc. A problem with testing a fish is that onCommand and onEvent do not
perform the operation directly, but merely return a description of the intended effect.

To test such functions, it is necessary to execute the effect in a test environment, preferably without interacting with real
external systems. To allow this, there are a [TestCommandExecutor](../testkit/testCommandExecutor.ts) and a 
[TestEventExector](../testkit/testEventExecutor.ts) in the testkit.

To allow testing entire fishes (onCommand and onEvent), there is an utility in the testkit called [FishTestFunctions](../testkit/FishTestFunctions.ts).
This allows transforming the onCommand and onEvent methods into methods that are more useful for testing.

When creating the fish test functions, it is possible to add deepFreeze. This should *only* be done in case object modification
is used anywhere inside the fish. Otherwise it is not necessary.

```typescript
const { onEvent, onCommand } = FishTestFunctions.of(productionProcessFish.type, { deepFreeze: true })
```

The result of executing onCommand with the test executor can be checked using a snapshot test

```typescript
describe('process onCommand', () => {
  it('should handle define', () => {
    expect(onCommand(is, defineCmd)).toMatchSnapshot()
  })
})
```

The result of onEvent can be either checked directly, or transformed to a public state to allow the internal state to change without breaking the test

```typescript
describe('process onEvent', () => {
  it('should handle defining processes', () => {
    const s = onEvent(is, mkEvent(definedEvent))
    return expect(pub(s)).resolves.toEqual(defined)
  })
})
```

When writing unit tests for fishes, it is often necessary to use the internal state of the fish. To have this information available,
fish tests should import the `FishTypeImpl<S, C, E, P>` of the fish.

# Chaos tests

todo

# Integration tests

todo

# Property based testing

todo
