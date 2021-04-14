/* eslint-disable @typescript-eslint/no-non-null-assertion, @typescript-eslint/no-var-requires */
const nodeSettingSchema = require('../../../../../protocols/json-schema/node-settings.schema.json')

import { readFile, remove } from 'fs-extra'
import { writeFile } from 'fs/promises'
import fetch from 'node-fetch'
import path from 'path'
import YAML from 'yaml'
import { assertOK } from '../../assertOK'
import { SettingsInput } from '../../cli/exec'
import { runOnEvery } from '../../infrastructure/hosts'
import { ActyxOSNode } from '../../infrastructure/types'
import { waitForNodeToBeConfigured } from '../../retry'
import { stubs } from '../../stubs'
import { createTestNodeDockerLocal } from '../../test-node-factory'

describe('ax settings (using quickstart ActyxOS default setting)', () => {
  const workingDir = '.'
  const settingDefaultFilePath = path.resolve(workingDir, 'fixtures/local-sample-node-settings.yml')
  const scopeActyxOS = 'com.actyx'

  let testNode: ActyxOSNode

  beforeAll(async () => {
    console.log('guess: ' + settingDefaultFilePath)

    // Node will be added to the global `thisEnvNodes` and eventually cleaned up
    testNode = await createTestNodeDockerLocal('settings')
  })

  const resetSettingActyxOS = async () => {
    await waitForNodeToBeConfigured(testNode)
    expect(await testNode.ax.settings.unset(scopeActyxOS)).toMatchCodeOk()
    await waitForNodeToBeConfigured(testNode)
    expect(
      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath)),
    ).toMatchCodeOk()
    await waitForNodeToBeConfigured(testNode)
    expect(
      await testNode.ax.settings.set(
        `${scopeActyxOS}/general/logLevels/os`,
        SettingsInput.FromValue('DEBUG'),
      ),
    ).toMatchCodeOk()
  }

  beforeEach(async () => {
    await resetSettingActyxOS()
    // wait for node to be configured. If we don't, restarting relevant services
    // inside the ActyxOS node might take too long on a busy test host,
    // otherwise deploying etc might not work below
    await waitForNodeToBeConfigured(testNode)
  })

  describe('scopes', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.unreachable.ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with default for com.actyx', async () => {
      await runOnEvery({}, async (node) => {
        const responses = assertOK(await node.ax.settings.scopes())
        expect(responses.result).toEqual(expect.arrayContaining([scopeActyxOS]))
      })
    })
  })

  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.unreachable.ax.settings.schema(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK for a valid ax schema for node', async () => {
      await runOnEvery({}, async (node) => {
        const response = assertOK(await node.ax.settings.schema(scopeActyxOS))
        expect(response.result).toMatchObject(nodeSettingSchema)
      })
    })

    // this will fail whenever we have unreleased changes — need to think about useful test
    test.skip('schema in docs is updated with cli schema', async () => {
      const urlSchemaWeb = 'https://developer.actyx.com/schemas/os/node-settings.schema.json'
      const responseWeb = await fetch(urlSchemaWeb)
      const schemaWeb = await responseWeb.json()

      const response = assertOK(await stubs.axOnly.ax.settings.schema(scopeActyxOS))
      expect(response.result).toMatchObject(schemaWeb)
    })
  })

  describe('get', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.unreachable.ax.settings.get(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with default node settings for com.actyx', async () => {
      const response = await testNode.ax.settings.get(scopeActyxOS)
      const responseShape = {
        code: 'OK',
        result: {
          general: {
            announceAddresses: [],
            authorizedKeys: [expect.any(String)],
            bootstrapNodes: [
              '/dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH',
            ],
            displayName: 'Local Sample Node',
            logLevels: {
              apps: 'INFO',
              os: 'DEBUG',
            },
            swarmKey:
              'L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK',
          },
          licensing: {
            apps: {},
            os: 'development',
          },
          services: {
            eventService: {
              readOnly: false,
              topic: 'SampleTopic',
            },
          },
        },
      }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and get specific properties from com.actyx setting', async () => {
      const responseDisplayName = await testNode.ax.settings.get('com.actyx/general/displayName')
      const responseDisplayNameeShape = { code: 'OK', result: 'Local Sample Node' }
      expect(responseDisplayName).toEqual(responseDisplayNameeShape)

      const responseLicense = await testNode.ax.settings.get('com.actyx/licensing')
      const responseLicenseShape = { code: 'OK', result: { apps: {}, os: 'development' } }
      expect(responseLicense).toEqual(responseLicenseShape)
    })

    test('return OK and show only properties added by the user on com.actyx setting --no-defaults', async () => {
      const settingCustomFilePath = path.resolve(
        workingDir,
        'fixtures/test-custom-actyxos-setting.yml',
      )

      await testNode.ax.settings.unset(scopeActyxOS)

      const doc = YAML.parseDocument(await readFile(settingDefaultFilePath, 'utf-8'))
      doc.setIn(['general', 'displayName'], 'Foo')

      await writeFile(settingCustomFilePath, doc.toString())

      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingCustomFilePath))

      const responseGet = assertOK(await testNode.ax.settings.get(scopeActyxOS))
      const responseGetNoDefaults = assertOK(await testNode.ax.settings.get(scopeActyxOS, true))
      expect(responseGetNoDefaults).not.toEqual(responseGet)
      expect(responseGetNoDefaults.result).not.toHaveProperty('announceAddresses')
      expect(responseGetNoDefaults.result).not.toHaveProperty('logLevels')

      await remove(settingCustomFilePath)
      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath))
    })

    test('return OK  with authorized key set if com.actyx has been unset', async () => {
      await testNode.ax.settings.unset(scopeActyxOS)

      const responseGet = await testNode.ax.settings.get(scopeActyxOS)
      expect(responseGet).toMatchCodeOk()
      expect(responseGet).toHaveProperty('result.general.authorizedKeys')

      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath))
    })
  })

  describe('unset', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.unreachable.ax.settings.unset(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK after unset com.actyx', async () => {
      const responseUnset = await testNode.ax.settings.unset(scopeActyxOS)
      const responseUnsetShape = { code: 'OK', result: {} }
      expect(responseUnset).toMatchObject(responseUnsetShape)
    })

    test('return OK for a not existing scope', async () => {
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
