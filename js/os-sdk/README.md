This is a JavaScript and TypeScript SDK for building ActyxOS apps. Currently, it wraps the ActyxOS Event Service and Console Service APIs in a simple client that can be used from within JavaScript and TypeScript apps.

# ActyxOS

For more information about ActyxOS, please check out our developer documentation at [https://developer.actyx.com/](https://developer.actyx.com/).

# Examples

## Event Service

### Subscribe to event streams

```typescript
import { Client, Subscription } from '@actyx/os-sdk'

const ActyxOS = Client()

// callback style
ActyxOS.eventService.subscribe({
  subscriptions: Subscription.everything(),
  onEvent: event => {
    console.log('got event', event)
  }
})

// stream style
const eventStream = ActyxOS.eventService.subscribeStream({
  subscriptions: Subscription.everything(),
})
for await (const event of eventStream) {
  console.log('got event', event)
}
```

### Publish events

```typescript
import { Client, EventDraft } from '@actyx/os-sdk'

const ActyxOS = Client()

// callback style
ActyxOS.eventService.publish({
  eventDrafts: EventDraft.make('mySemantics', 'myName', { foo: 'bar' }),
  onDone: () => {
    console.log(`Published`)
  }
})

// promise style
await ActyxOS.eventService.publishPromise({
  eventDrafts: EventDraft.make('mySemantics', 'myName', { foo: 'bar' }),
})
```

## Console Service

### Simple logging

For simple logging needs, use the `SimpleLogger` which you can configure once
and then use to log messages and, optionally, additional data:

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

### Advanced (custom) logging

For more advanced, custom logging needs, use the log function directly:

```typescript
import { Client } from '@actyx/os-sdk'

const ActyxOS = Client()

ActyxOS.consoleService.log({
  entry: {
    logName: 'myCustomLogger',
    message: 'this is a WARNING message',
    severity: LogSeverity.WARN,
    producer: {
      name: 'com.example.app1',
      version: '1.0.0'
    },
    additionalData: {
      foo: 'bar',
      bar: {
        foo: true,
      }
    },
    labels: {
      'com.example.app1.auth.username': 'john.doe',
      'com.example.app1.model.events': '10000',
    }
  },
  // Callback on successful logging
  onLogged: () => {
    // Do something
  },
  // Callback on error logging
  onError: err => {
    console.error(`error logging: ${err}`)
  }
})
```

There also is a corresponding version that returns a `Promise`:

```typescript
import { Client } from '@actyx/os-sdk'

const ActyxOS = Client()

await ActyxOS.consoleService.logPromise({ /* log entry */ })
```

# Usage

## Installation

Install via npm using

```
npm install @actyx/os-sdk
```

## Documentation

You can access the Typedoc documentation at https://developer.actyx.com/@actyx/os-sdk/.

## Getting help

If you have questions about or issues with the ActyxOS SDK, join our [Discord chat](https://discord.gg/262yJhc) or email us at developer@actyx.io.

## Advanced

### Overriding the default client options

You can override the default client options by passing options ([`ApiClientOpts`](https://developer.actyx.com/@actyx/os-sdk/interfaces/apiclientopts.html)) when creating the client. If you only want to override specific options, use the default options ([`DefaultClientOpts`](https://developer.actyx.com/@actyx/os-sdk/globals.html#defaultclientopts)) as shown in the following example:

```typescript
import { Client, DefaultClientOpts } from '@actyx/os-sdk'

const clientOpts = DefaultClientOpts()
clientOpts.Endpoints.EventService.BaseUrl = 'http://10.2.3.23:4454/api'

const CustomActyxOS = Client(clientOpts)
```

## Development

- Build with `npm run build`
- Run tests with `npm run test`
- Run test including integration tests with `RUN_INTEGRATION_TESTS=1 npm run test`
- Run lint / lint fix with `npm run lint` and `npm run lint:fix`
- Publish with `npm publish`
