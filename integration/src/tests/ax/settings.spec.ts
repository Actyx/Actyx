/* eslint-disable @typescript-eslint/no-non-null-assertion, @typescript-eslint/no-var-requires */
const nodeSettingSchema = require('../../../../protocols/json-schema/node-settings.schema.json')

import { assertOK } from '../../assertOK'
import { CLI } from '../../cli'
import { SettingsInput } from '../../cli/exec'
import { getFreePort } from '../../infrastructure/checkPort'
import { runOnEvery } from '../../infrastructure/hosts'
import { currentAxBinary } from '../../infrastructure/settings'
import { newProcess } from '../../util'

describe('ax settings', () => {
  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const ax = await CLI.build(`localhost:${await getFreePort()}`, await currentAxBinary())
      const response = await ax.settings.schema()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK for a valid ax schema for node', () =>
      runOnEvery(async (node) => {
        const response = assertOK(await node.ax.settings.schema())
        expect(response.result).toMatchObject(nodeSettingSchema)
      }))
  })

  describe('get', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const ax = await CLI.build(`localhost:${await getFreePort()}`, await currentAxBinary())
      const response = await ax.settings.get('/')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with default node settings', () =>
      runOnEvery(async (node) => {
        const response = await node.ax.settings.get('/')
        const responseShape = {
          code: 'OK',
          result: {
            admin: {
              authorizedUsers: expect.arrayContaining([]),
              displayName: expect.any(String),
              logLevels: {
                node: 'DEBUG',
              },
            },
            api: {
              events: {
                readOnly: false,
              },
            },
            licensing: {
              apps: expect.objectContaining({}),
              node: 'development',
            },
            swarm: {
              announceAddresses: [],
              initialPeers: expect.arrayContaining([]),
              swarmKey: expect.any(String),
            },
          },
        }
        expect(response).toMatchObject(responseShape)
      }))

    test('return OK and get specific properties', () =>
      runOnEvery(async (node) => {
        expect(await node.ax.settings.get('/admin/displayName')).toEqual({
          code: 'OK',
          result: expect.any(String),
        })

        expect(await node.ax.settings.get('/licensing')).toEqual({
          code: 'OK',
          result: { apps: expect.objectContaining({}), node: 'development' },
        })
      }))
  })

  describe('unset', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const ax = await CLI.build(`localhost:${await getFreePort()}`, await currentAxBinary())
      const response = await ax.settings.unset('/')
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK after unset', () =>
      runOnEvery(async (node) => {
        if (node.host !== 'process') {
          return
        }
        const n = await newProcess(node)

        try {
          expect(await n.ax.settings.unset('/')).toEqual({
            code: 'OK',
            result: { scope: '/' },
          })
          expect(await n.ax.settings.unset('/wat')).toEqual({
            code: 'OK',
            result: { scope: '/wat' },
          })
        } finally {
          n._private.shutdown()
        }
      }))
  })

  describe('set', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const ax = await CLI.build(`localhost:${await getFreePort()}`, await currentAxBinary())
      const response = await ax.settings.set('/asdf', SettingsInput.FromValue(42))
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK for valid setting', () =>
      runOnEvery(async (node) => {
        if (node.host !== 'process') {
          return
        }
        const n = await newProcess(node)
        try {
          const result = assertOK(
            await n.ax.settings.set('/admin/displayName', SettingsInput.FromValue('minime')),
          ).result
          expect(result.scope).toBe('/admin/displayName')
          expect(result.settings).toBe('minime')
        } finally {
          n._private.shutdown()
        }
      }))

    test('return error for invalid setting', () =>
      runOnEvery(async (node) => {
        if (node.host !== 'process') {
          return
        }
        const n = await newProcess(node)
        try {
          expect(
            await n.ax.settings.set('/admin/displayName', SettingsInput.FromValue(57)),
          ).toMatchObject({
            code: 'ERR_SETTINGS_INVALID',
            message: expect.stringContaining('The value must be string.'),
          })
        } finally {
          n._private.shutdown()
        }
      }))
  })
})
