---
title: JS/TS SDK
---

<!-- Add as react component to be able to handle the width (otherwise it goes full width) -->
<img src="/images/js-sdk.png" style={{maxWidth: "150px", marginBottom: "2rem" }} />

Building apps in [JavaScript](https://developer.mozilla.org/en-US/docs/Web/JavaScript) or [TypeScript](https://www.typescriptlang.org/) and want to easily create and access data streams in your ActyxOS swarm? Or you want to log from your app to easily access logs using the Actyx CLI? That's what we built the ActyxOS SDK for JavaScript/TypeScript for. The [`@actyx/os-sdk` package](http://npmjs.com/package/@actyx/os-sdk) defines all necessary data types and provides bindings for communicating with ActyxOS's [Event Service API](../api/event-service.md) and [Console Service API](../api/console-service.md).

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

This is how you could log things from within your app:

```typescript
import { Client } from '@actyx/os-sdk'

const ActyxOS = Client()

const logger: SimpleLogger = ActyxOS.consoleService.SimpleLogger({
  logName: 'myLogger',
  producerName: 'com.example.app1',
  producerVersion: '1.0.0'
})

logger.debug('this is a DEBUG message')
logger.warn('this is a WARNING message')
logger.info('this is an INFO message')
logger.error('this is an ERROR message')

logger.debug('This is a message with additional data', {foo: 'bar'})
```

## Documentation

Check out the latest <a href="/@actyx/os-sdk" target="_blank" rel="noopener noreferrer">documentation for the JS/TS SDK</a>.
