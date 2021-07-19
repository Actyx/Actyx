/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { Actyx } from '@actyx/sdk'
import { runOnEvery } from '../../infrastructure/hosts'

describe('@actyx/sdk', () => {
  test('node unreachable', async () => {
    await runOnEvery(async (_node) => {
      const wrongConn = Actyx.of(null!)

      await expect(wrongConn).rejects.toMatchObject({
        message:
          'Error: unable to connect to Actyx at http://localhost:4454/api/v2/node/id. Is the service running? -- Error: request to http://localhost:4454/api/v2/node/id failed, reason: connect ECONNREFUSED 127.0.0.1:4454',
      })
    })
  })

  test('connection without manifest (hello JS users)', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(null!, {
        actyxPort: node._private.apiEventsPort,
      })

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with missing manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
        },
        {
          actyxPort: node._private.apiEventsPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with super bad manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
          signature: 'garbage',
        },
        {
          actyxPort: node._private.apiEventsPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
      })
    })
  })

  test('connection with invalid manifest signature', async () => {
    await runOnEvery(async (node) => {
      const wrongConn = Actyx.of(
        {
          appId: 'bad.example.bad-app',
          displayName: 'My Example App',
          version: '1.0.0',
          signature:
            'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
        },
        {
          actyxPort: node._private.apiEventsPort,
        },
      )

      await expect(wrongConn).rejects.toMatchObject({
        message:
          'Invalid manifest. AppId \'bad.example.bad-app\' is not allowed in app_domains \'[AppDomain("com.actyx.*"), AppDomain("com.example.*")]\'',
      })
    })
  })
})
