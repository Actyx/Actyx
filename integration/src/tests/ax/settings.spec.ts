// eslint-disable-next-line @typescript-eslint/no-var-requires
const nodeSettingSchema = require('../../../../../protocols/json-schema/os/node-settings.schema.json')

import { stubs } from '../../stubs'
import fetch from 'node-fetch'
import { assertOK } from '../../assertOK'
import { runOnEvery } from '../../infrastructure/hosts'

describe('ax settings', () => {
  describe('scopes', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return default com.actyx.os', async () => {
      await runOnEvery({}, async (node) => {
        const responses = assertOK(await node.ax.settings.scopes())
        expect(responses.result).toEqual(expect.arrayContaining(['com.actyx.os']))
      })
    })
  })

  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.settings.schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return valid ax schema for node', async () => {
      await runOnEvery({}, async (node) => {
        const response = assertOK(await node.ax.settings.schema('com.actyx.os'))
        expect(response.result).toMatchObject(nodeSettingSchema)
      })
    })

    // this will fail whenever we have unreleased changes — need to think about useful test
    test.skip('schema in docs is updated with cli schema', async () => {
      const urlSchemaWeb = 'https://developer.actyx.com/schemas/os/node-settings.schema.json'
      const responseWeb = await fetch(urlSchemaWeb)
      const schemaWeb = await responseWeb.json()

      const response = assertOK(await stubs.axOnly.ax.settings.schema('com.actyx.os'))
      expect(response.result).toMatchObject(schemaWeb)
    })
  })
})
