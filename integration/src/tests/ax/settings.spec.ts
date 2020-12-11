/* eslint-disable @typescript-eslint/no-non-null-assertion, @typescript-eslint/no-var-requires */
const nodeSettingSchema = require('../../../../../protocols/json-schema/os/node-settings.schema.json')

import { stubs } from '../../stubs'
import fetch from 'node-fetch'
import { assertOK } from '../../assertOK'
import { runOnEvery } from '../../infrastructure/hosts'
import { ActyxOSNode } from '../../infrastructure/types'
import { createPackageSampleDockerApp, createTestNodeDockerLocal } from '../../test-node-factory'
import { readFile, remove } from 'fs-extra'
import { quickstartDirs } from '../../setup-projects/quickstart'
import { settings } from '../../infrastructure/settings'
import path from 'path'
import YAML from 'yaml'
import { writeFile } from 'fs/promises'
import { SettingsInput } from '../../cli/exec'
import { waitForAppToStart, waitForAppToStop } from '../../retry'

describe('ax settings (using quickstart ActyxOS default setting)', () => {
  const workingDir = quickstartDirs(settings().tempDir).quickstart
  const settingDefaultFilePath = path.resolve(workingDir, 'misc/local-sample-node-settings.yml')
  const scopeActyxOS = 'com.actyx.os'

  let testNode: ActyxOSNode
  let packagePath = ''
  let appId = ''

  beforeAll(async () => {
    testNode = await createTestNodeDockerLocal('settings')

    const infoSampleDockerApp = await createPackageSampleDockerApp(testNode)
    packagePath = infoSampleDockerApp.packagePath
    appId = infoSampleDockerApp.appId
  })

  const resetSettingActyxOS = async () => {
    await testNode.ax.settings.unset(scopeActyxOS)
    await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath))
  }

  beforeEach(async () => await resetSettingActyxOS())

  describe('scopes', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.scopes()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with default for com.actyx.os', async () => {
      await runOnEvery({}, async (node) => {
        const responses = assertOK(await node.ax.settings.scopes())
        expect(responses.result).toEqual(expect.arrayContaining([scopeActyxOS]))
      })
    })
  })

  describe('schema', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.settings.schema(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.schema(scopeActyxOS)
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
      const response = await stubs.hostUnreachable.ax.settings.get(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.get(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with default node settings for com.actyx.os', async () => {
      const response = await testNode.ax.settings.get(scopeActyxOS)
      const responseShape = {
        code: 'OK',
        result: {
          general: {
            announceAddresses: [],
            authorizedKeys: [],
            bootstrapNodes: [
              '/dns4/demo-bootstrap.actyx.net/tcp/4001/ipfs/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH',
            ],
            displayName: 'Local Sample Node',
            logLevels: {
              apps: 'INFO',
              os: 'INFO',
            },
            requireAuthentication: false,
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

    test('return OK and get specific properties from com.actyx.os setting', async () => {
      const responseDisplayName = await testNode.ax.settings.get('com.actyx.os/general/displayName')
      const responseDisplayNameeShape = { code: 'OK', result: 'Local Sample Node' }
      expect(responseDisplayName).toEqual(responseDisplayNameeShape)

      const responseLicense = await testNode.ax.settings.get('com.actyx.os/licensing')
      const responseLicenseShape = { code: 'OK', result: { apps: {}, os: 'development' } }
      expect(responseLicense).toEqual(responseLicenseShape)
    })

    test('return OK and show only properties added by the user on com.actyx.os setting --no-defaults', async () => {
      const settingCustomFilePath = path.resolve(workingDir, 'misc/test-custom-actyxos-setting.yml')

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

    test('return ERR_INTERNAL_ERROR if com.actyx.os is not set --no-defaults', async () => {
      await testNode.ax.settings.unset(scopeActyxOS)

      const responseGet = await testNode.ax.settings.get(scopeActyxOS, true)
      const responseGetShape = {
        code: 'ERR_SETTINGS_NOT_FOUND_AT_SCOPE',
        message: expect.any(String),
      }
      expect(responseGet).toMatchObject(responseGetShape)
      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath))
    })

    test('return OK if com.actyx.os is not set', async () => {
      await testNode.ax.settings.unset(scopeActyxOS)

      const responseGet = await testNode.ax.settings.get(scopeActyxOS)
      expect(responseGet).toMatchCodeOk()

      await testNode.ax.settings.set(scopeActyxOS, SettingsInput.FromFile(settingDefaultFilePath))
    })
  })

  describe.skip('unset', () => {
    test('return ERR_NODE_UNREACHABLE if node host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.settings.unset(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.settings.unset(scopeActyxOS)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK after unset com.actyx.os', async () => {
      const responseUnset = await testNode.ax.settings.unset(scopeActyxOS)
      const responseUnsetShape = { code: 'OK', result: {} }
      expect(responseUnset).toMatchObject(responseUnsetShape)
    })

    test('return ERR_SETTINGS_UNKNOWN_SCOPE after unset or a not existing scope', async () => {
      const responseUnset = await testNode.ax.settings.unset('not-existing-scope')
      const responseUnsetShape = {
        code: 'ERR_SETTINGS_UNKNOWN_SCOPE',
        message: expect.any(String),
      }
      expect(responseUnset).toMatchObject(responseUnsetShape)
    })

    test('return OK after unset setting for an app', async () => {
      await testNode.ax.apps.deploy(packagePath)

      const response = await testNode.ax.settings.unset(appId)
      const responseShape = { code: 'OK', result: {} }
      expect(response).toEqual(responseShape)

      await testNode.ax.apps.undeploy(appId)
    })

    test('return ERR_APP_ENABLED if app is running', async () => {
      await testNode.ax.apps.deploy(packagePath)

      await testNode.ax.apps.start(appId)
      await waitForAppToStart(appId, testNode)

      const response = await testNode.ax.settings.unset(appId)
      const responseShape = { code: 'ERR_APP_ENABLED', message: expect.any(String) }
      expect(response).toMatchObject(responseShape)

      await testNode.ax.apps.stop(appId)
      await waitForAppToStop(appId, testNode)

      await testNode.ax.apps.undeploy(appId)
    })

    test('return ERR_APP_ENABLED if app is running and unset is for com.actyx.os', async () => {
      await testNode.ax.apps.deploy(packagePath)

      await testNode.ax.apps.start(appId)
      await waitForAppToStart(appId, testNode)

      const response = await testNode.ax.settings.unset(scopeActyxOS)
      const responseShape = { code: 'ERR_APP_ENABLED', message: expect.any(String) }
      expect(response).toMatchObject(responseShape)

      await testNode.ax.apps.stop(appId)
      await waitForAppToStop(appId, testNode)

      await testNode.ax.apps.undeploy(appId)
    })
  })
})
