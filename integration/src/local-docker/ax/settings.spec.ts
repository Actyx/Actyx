// eslint-disable-next-line @typescript-eslint/no-var-requires
const nodeSettingSchema = require('../../../../../protocols/json-schema/os/node-settings.schema.json')

import { stubNode, stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../../stubs'
import fetch from 'node-fetch'
import { resetTestEviroment } from '../local-docker-util'

describe('ax settings', () => {
  beforeAll(async () => {
    await resetTestEviroment()
  })
  afterAll(async () => {
    await resetTestEviroment()
  })

  describe('scopes', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubNodeHostUnreachable.ax.Settings.Scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Settings.Scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return default com.actyx.os', async () => {
      const responses = await stubNode.ax.Settings.Scopes()
      const responsesShape = { code: 'OK', result: ['com.actyx.os'] }
      expect(responses).toMatchObject(responsesShape)
    })
  })

  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubNodeHostUnreachable.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchErrNodeUnreachable()
    })

    // TODO: enable this test later when we can compare with the latest schema,
    // schema from actyxos-linux seems not updated
    test.skip('return valid ax schema for node with no apps', async () => {
      const response = await stubNode.ax.Settings.Schema('com.actyx.os')
      expect(response).toMatchCodeOk()
      expect(response).toMatchObject(nodeSettingSchema)
    })

    // TODO: enable this test later when we can compare with the latest schema
    test.skip('schema in docs is updated with cli schema', async () => {
      const urlSchemaWeb = 'https://developer.actyx.com/schemas/os/node-settings.schema.json'
      const responseWeb = await fetch(urlSchemaWeb)
      const schemaWeb = await responseWeb.json()

      const response = await stubNode.ax.Settings.Schema('com.actyx.os')
      const schemaCli = response.code === 'OK' && response.result
      expect(response).toMatchCodeOk()
      expect(schemaCli).toMatchObject(schemaWeb)
    })
  })
})
