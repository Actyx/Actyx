This is a JavaScript and TypeScript SDK for building ActyxOS apps. Currently, it wraps the ActyxOS Event Service API in a simple client that can be used from within JavaScript and TypeScript apps.

# ActyxOS

For more information about ActyxOS, please check out our developer documentation at [https://developer.actyx.com/](https://developer.actyx.com/).

# Examples

## Subscribe to event streams

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

## Publish events

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

# Usage

## Installation

Install via npm using

```
npm install @actyx/os-sdk
```

## Documentation

You can access the Typedoc documentation at https://developer.actyx.com/@actyx/os-sdk/. See especially the documentation for the [offsets](https://developer.actyx.com/@actyx/os-sdk/interfaces/eventserviceclient.html#offsets), [query](https://developer.actyx.com/@actyx/os-sdk/interfaces/eventserviceclient.html#query), [subscribe](https://developer.actyx.com/@actyx/os-sdk/interfaces/eventserviceclient.html#subscribe), and [publish](https://developer.actyx.com/@actyx/os-sdk/interfaces/eventserviceclient.html#publish) functions.

## Getting help

If you have questions about or issues with the ActyxOS SDK, join our [Discord chat](https://discord.gg/262yJhc) or email us at developer@actyx.io.

## Advanced

### Overriding the default client options

You can override the default client options by passing options ([`ApiClientOpts`](https://developer.actyx.com/@actyx/os-sdk/interfaces/apiclientopts.html)) when creating the client. If you only want to override specific options, use the default options ([`DefaultClientOpts`](https://developer.actyx.com/@actyx/os-sdk/globals.html#defaultclientopts)) as shown in the following example:

```typescript
import { Client, DefaultClientOpts } from '@actyx/os-sdk'

const CustomActyxOS = Client({
  ...DefaultClientOpts,
  Ports: {
    EventService: 5555
  }
})
```

## Development

- Build with `npm run build`
- Run tests with `npm run test`
- Run test including integration tests with `RUN_INTEGRATION_TESTS=1 npm run test`
- Run lint / lint fix with `npm run lint` and `npm run lint:fix`
- Publish with `npm publish`
