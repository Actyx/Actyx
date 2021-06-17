/* eslint-disable @typescript-eslint/no-non-null-assertion, @typescript-eslint/no-var-requires */
const nodeSettingSchema = require('../../../../protocols/json-schema/node-settings.schema.json')

import { readFile, remove } from 'fs-extra'
import { writeFile } from 'fs/promises'
import fetch from 'node-fetch'
import path from 'path'
import YAML from 'yaml'
import { assertOK } from '../../assertOK'
import { CLI } from '../../cli'
import { SettingsInput } from '../../cli/exec'
import { runOnEvery } from '../../infrastructure/hosts'
import { ActyxNode } from '../../infrastructure/types'
import { waitForNodeToBeConfigured } from '../../retry'
import { mkAx, mkAxWithUnreachableNode } from '../../stubs'
import { createTestNodeLocal } from '../../test-node-factory'

describe('ax settings', () => {
  const workingDir = '.'
  const settingDefaultFilePath = path.resolve(workingDir, 'fixtures/local-sample-node-settings.yml')
  const scopeActyx = 'com.actyx'

  let testNode: ActyxNode
  let ax: CLI

  beforeAll(async () => {
    // Node will be added to the global `thisEnvNodes` and eventually cleaned up
    if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
      return
    }
    testNode = await createTestNodeLocal('settings')
    ax = await mkAxWithUnreachableNode()
  })

  const resetSettingActyx = async () => {
    await waitForNodeToBeConfigured(testNode)
    expect(await testNode.ax.settings.unset(scopeActyx)).toMatchCodeOk()
    await waitForNodeToBeConfigured(testNode)
    expect(
      await testNode.ax.settings.set(scopeActyx, SettingsInput.FromFile(settingDefaultFilePath)),
    ).toMatchCodeOk()
    await waitForNodeToBeConfigured(testNode)
    expect(
      await testNode.ax.settings.set(
        `${scopeActyx}/admin/logLevels/node`,
        SettingsInput.FromValue('DEBUG'),
      ),
    ).toMatchCodeOk()
  }

  beforeEach(async () => {
    if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
      return
    }
    await resetSettingActyx()
    // wait for node to be configured. If we don't, restarting relevant services
    // inside the Actyx node might take too long on a busy test host,
    // otherwise deploying etc might not work below
    await waitForNodeToBeConfigured(testNode)
  })

  describe('scopes', () => {
    // FIXME: doesn't work on Windows
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const response = await ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    // FIXME: doesn't work on Windows
    test('return OK with default for com.actyx', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      await runOnEvery(async (node) => {
        const responses = assertOK(await node.ax.settings.scopes())
        expect(responses.result).toEqual(expect.arrayContaining([scopeActyx]))
      })
    })
  })

  describe('schema', () => {
    // FIXME: doesn't work on Windows
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const response = await ax.settings.schema(scopeActyx)
      expect(response).toMatchErrNodeUnreachable()
    })

    // FIXME: doesn't work on Windows
    test('return OK for a valid ax schema for node', async () => {
      await runOnEvery(async (node) => {
        if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
          return
        }
        const response = assertOK(await node.ax.settings.schema(scopeActyx))
        expect(response.result).toMatchObject(nodeSettingSchema)
      })
    })

    // FIXME: doesn't work on Windows
    // this will fail whenever we have unreleased changes — need to think about useful test
    test.skip('schema in docs is updated with cli schema', async () => {
      const urlSchemaWeb = 'https://developer.actyx.com/schemas/os/node-settings.schema.json'
      const responseWeb = await fetch(urlSchemaWeb)
      const schemaWeb = await responseWeb.json()

      const axOnly = await mkAx()
      const response = assertOK(await axOnly.settings.schema(scopeActyx))
      expect(response.result).toMatchObject(schemaWeb)
    })
  })

  describe('get', () => {
    // FIXME: doesn't work on Windows
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const response = await ax.settings.get(scopeActyx)
      expect(response).toMatchErrNodeUnreachable()
    })

    // FIXME: doesn't work on Windows
    test('return OK with default node settings for com.actyx', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const response = await testNode.ax.settings.get(scopeActyx)
      const responseShape = {
        code: 'OK',
        result: {
          admin: {
            authorizedUsers: [expect.any(String)],
            displayName: 'Local Sample Node',
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
            apps: {},
            node: 'development',
          },
          swarm: {
            announceAddresses: [],
            initialPeers: [
              '/dns4/demo-bootstrap.actyx.net/tcp/4001/p2p/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH',
            ],
            swarmKey: 'MDAwMDAwMDAxMTExMTExMTIyMjIyMjIyMzMzMzMzMzM=',
          },
        },
      }
      expect(response).toMatchObject(responseShape)
    })

    // FIXME: doesn't work on Windows
    test('return OK and get specific properties from com.actyx setting', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const responseDisplayName = await testNode.ax.settings.get('com.actyx/admin/displayName')
      const responseDisplayNamedShape = { code: 'OK', result: 'Local Sample Node' }
      expect(responseDisplayName).toEqual(responseDisplayNamedShape)

      const responseLicense = await testNode.ax.settings.get('com.actyx/licensing')
      const responseLicenseShape = { code: 'OK', result: { apps: {}, node: 'development' } }
      expect(responseLicense).toEqual(responseLicenseShape)
    })

    // FIXME: doesn't work on Windows
    test.skip('return OK and show only properties added by the user on com.actyx setting --no-defaults', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const settingCustomFilePath = path.resolve(
        workingDir,
        'fixtures/test-custom-actyx-setting.yml',
      )

      await testNode.ax.settings.unset(scopeActyx)

      const doc = YAML.parseDocument(await readFile(settingDefaultFilePath, 'utf-8'))
      doc.setIn(['admin', 'displayName'], 'Foo')

      await writeFile(settingCustomFilePath, doc.toString())

      await testNode.ax.settings.set(scopeActyx, SettingsInput.FromFile(settingCustomFilePath))

      const responseGet = assertOK(await testNode.ax.settings.get(scopeActyx))
      const responseGetNoDefaults = assertOK(await testNode.ax.settings.get(scopeActyx, true))
      expect(responseGetNoDefaults).not.toEqual(responseGet)
      expect(responseGetNoDefaults.result).not.toHaveProperty('announceAddresses')
      expect(responseGetNoDefaults.result).not.toHaveProperty('logLevels')

      await remove(settingCustomFilePath)
      await testNode.ax.settings.set(scopeActyx, SettingsInput.FromFile(settingDefaultFilePath))
    })

    // FIXME: doesn't work on Windows
    test('return OK with authorized key set if com.actyx has been unset', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      await testNode.ax.settings.unset(scopeActyx)

      const responseGet = await testNode.ax.settings.get(scopeActyx)
      expect(responseGet).toMatchCodeOk()
      expect(responseGet).toHaveProperty('result.admin.authorizedUsers')

      await testNode.ax.settings.set(scopeActyx, SettingsInput.FromFile(settingDefaultFilePath))
    })
  })

  describe('unset', () => {
    // FIXME: doesn't work on Windows
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const response = await ax.settings.unset(scopeActyx)
      expect(response).toMatchErrNodeUnreachable()
    })

    // FIXME: doesn't work on Windows
    test('return OK after unset com.actyx', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const responseUnset = await testNode.ax.settings.unset(scopeActyx)
      const responseUnsetShape = { code: 'OK', result: {} }
      expect(responseUnset).toMatchObject(responseUnsetShape)
    })

    // FIXME: doesn't work on Windows
    test('return OK for a not existing scope', async () => {
      if (process.platform === 'win32') { // to unblock running on Windows (will be fixed by removing createTestNodeLocal)
        return
      }
      const scope = 'i-dont-exist'
      const responseUnset = await testNode.ax.settings.unset(scope)
      const responseUnsetShape = {
        code: 'OK',
        result: { scope },
      }
      expect(responseUnset).toMatchObject(responseUnsetShape)
    })
  })
})
