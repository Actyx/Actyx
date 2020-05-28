---
title: JS/TS SDK
---

<!-- Add as react component to be able to handle the width (otherwise it goes full width) -->
<img src="/images/js-sdk.png" style={{maxWidth: "150px", marginBottom: "2rem" }} />

Building apps in [Javascript](https://developer.mozilla.org/en-US/docs/Web/JavaScript) or [Typescript](https://www.typescriptlang.org/) and want to easily create and access data streams in your ActyxOS swarm? That's what we built the ActyxOS SDK for Javascript/Typescript for. The [`@actyx/os-sdk` package](http://npmjs.com/package/@actyx/os-sdk) defines all necessary data types and provides bindings for communicating with ActyxOS's [Event Service API](../api/event-service.md).


## Installation

Install with npm as follows:

```bash
npm install @actyx/os-sdk
```

## Example

Here is an example using the SDK to subscribe to an event stream:

```typescript
import { Client, Subscription } from '@actyx/os-sdk'

const ActyxOS = Client()

ActyxOS.eventService.subscribe({
  subscriptions: Subscription.everything(),
  onEvent: event => {
    console.log(`got event: ${JSON.stringify(event)}`)
  }
})
```

Here is how you would publish events:

```typescript
import { Client, EventDraft } from '@actyx/os-sdk'

const ActyxOS = Client()

ActyxOS.eventService.publish({
  eventDrafts: EventDraft.make('mySemantics', 'myName', { foo: 'bar' }),
  onDone: () => {
    console.log(`Published`)
  }
})
```

## Documentation

Check out the <a href="/@actyx/os-sdk" target="_blank" rel="noopener noreferrer">automatically generated Typedocs.</a>.

