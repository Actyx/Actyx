<div align="center">
    <h1>Actyx Typescript/Javascript SDK</h1>
    <a href="https://www.npmjs.com/package/@actyx/sdk"><img src="https://img.shields.io/npm/v/@actyx/sdk.svg?style=flat" /></a>
    <a href="https://github.com/Actyx/Actyx/blob/master/README.md#contributing"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" /></a>
    <br />
    <hr />
</div>

A open-source Typescript/Javascript SDK for interacting the [Actyx APIs](https://developer.actyx.com/docs/reference/overview):

- Emit, query, and subscribe to events that get distributed via Actyx
- Get Actyx diagnostics
- Scoped to your custom app id

This SDK is the basis for the more frequently used [Actyx Pond](https://developer.actyx.com/docs/how-to/actyx-pond/introduction) framework.

## Example usage

```typescript
import { Actyx, Tags } from '@actyx/sdk'

(async () => {

    // Connect to the local Actyx process
    const actyx = await Actyx.of({
        appId: 'com.example.app',
        displayName: 'Example App',
        version: '1.0.0'
    })

    // Get latest event stream offsets
    const offsets = await actyx.offsets()
    console.log(offsets)

    // Emit events
    await actyx.emit([
        {
            tags: ['tag-1', 'tag-2'],
            event: {
                foo: 'bar'
            }
        }
    ])

    // Subscribe to events
    await actyx.subscribe({
        query: Tags('tag-1').and('tag-2')
    }, event => {
        console.log(event)
    })
})()
```
