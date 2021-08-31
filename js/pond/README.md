<div align="center">
    <h1>Actyx Pond Framework</h1>
    <a href="https://www.npmjs.com/package/@actyx/pond"><img src="https://img.shields.io/npm/v/@actyx/pond.svg?style=flat" /></a>
    <a href="https://github.com/Actyx/Actyx/blob/master/README.md#contributing"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" /></a>
    <br />
    <hr />
</div>

An open-source Typescript/Javascript framework for implementing distributed state-machines which are automatically kept in sync across 
a swarm of interconnected devices. The Actyx Pond requires Actyx to be running on each device.

The key features of Actyx Pond are:

- **Distributed event-sourcing** for great information replication facilities and declarative information consumption
- **Partition tolerance** with an eventually consistent programming model for arbitrary business logic
- **Eventual consistency** by using a state machine time-travel algorithm to agree on global state

This package builds on the [Actyx SDK](https://www.npmjs.com/package/@actyx/sdk).

## Example usage

```typescript
import { Pond, Tag, Fish, FishId } from '@actyx/pond'
(async () => {

    // Connect to the local Actyx process
    const pond = await Pond.default({
        appId: 'com.example.app',
        displayName: 'Example App',
        version: '1.0.0'
    })


    const chatTag = Tag<string>('ChatMessage');
    // A fish is a state machine
    const ChatFish: Fish<string[], string> = {
        fishId: FishId.of('chat', 'MyChatFish', 0),
        initialState: [],
        onEvent: (state, event) => {
            state.push(event);
            return state;
        },
        where: chatTag,
    };

    // Example event emission; this can actually
    // happen on any node running Actyx
    setInterval(() => {
        pond.emit(chatTag, 'a chat message')
    }, 2_000)

    // Observe time-travelling state machine
    pond.observe(ChatFish, (state) => {
        console.log(state)
    })

})()
```

## Recommended VSCode plugins
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint) for live source code linting
