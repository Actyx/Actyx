/* eslint-disable @typescript-eslint/no-non-null-assertion */
import * as path from 'path'
import { stubs } from '../../stubs'
import { remove, pathExists, ensureFile } from 'fs-extra'
import { quickstartDirs } from '../../setup-projects/quickstart'
import { demoMachineKitDirs } from '../../setup-projects/demo-machine-kit'
import { assertOK } from '../../assertOK'
import { ActyxOSNode } from '../../infrastructure/types'
import { waitFor, waitForAppToStart, waitForAppToStop } from '../../retry'
import { settings } from '../../infrastructure/settings'
import { createPackageSampleDockerApp, createTestNodeDockerLocal } from '../../test-node-factory'
import { tempDir } from '../../setup-projects/util'

describe('ax apps', () => {
  const invalidPath = 'invalid-path'
  const projectTempDir = path.resolve(settings().tempDir)

  let testNode: ActyxOSNode

  const sampleWebviewAppDir = quickstartDirs(projectTempDir).sampleWebviewApp
  const workingDir = tempDir()

  beforeAll(async () => {
    testNode = await createTestNodeDockerLocal('apps')
  })

  describe('ls', () => {
    test('return ERR_NODE_UNREACHABLE if host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.apps.ls()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.apps.ls()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with an empty list', async () => {
      const responses = assertOK(await testNode.ax.apps.ls())
      expect(responses).toEqual(expect.arrayContaining([]))
    })
  })

  describe('validate', () => {
    test('return ERR_INVALID_INPUT if file path does not exist', async () => {
      const response = await testNode.ax.apps.validate(invalidPath)
      expect(response).toMatchErrInvalidInput()
    })

    test('return ERR_INVALID_INPUT if file paths do not exist for multiple apps', async () => {
      const response = await testNode.ax.apps.validateMultiApps([
        `${invalidPath}1`,
        `${invalidPath}2`,
      ])
      expect(response).toMatchErrInvalidInput()
    })

    test('return OK and validate an app in the specified directory with default manifest', async () => {
      const response = await testNode.ax.apps.validate(sampleWebviewAppDir)
      const responseShape = { code: 'OK', result: [sampleWebviewAppDir] }
      expect(response).toEqual(responseShape)
    })

    test('return OK and validate an app with default manifest', async () => {
      const response = await testNode.ax.apps.validateCwd(sampleWebviewAppDir)
      const responseShape = { code: 'OK', result: ['ax-manifest.yml'] }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${sampleWebviewAppDir}/ax-manifest.yml`
      const response = await testNode.ax.apps.validate(manifestPath)
      const responseShape = { code: 'OK', result: [manifestPath] }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and validate apps if file paths do exists', async () => {
      const response = await testNode.ax.apps.validateMultiApps([
        demoMachineKitDirs(projectTempDir).dashboard,
        demoMachineKitDirs(projectTempDir).erpSimulator,
      ])
      const responseShape = {
        code: 'OK',
        result: [
          'temp/DemoMachineKit/src/dashboard',
          'temp/DemoMachineKit/src/erp-simulator',
        ].map((d) => path.resolve(d)),
      }
      expect(response).toMatchObject(responseShape)
    })

    test('return ERR_INVALID_INPUT if manifest is invalid', async () => {
      const manifestPath = `${sampleWebviewAppDir}/invalid-manifest.yml`
      await ensureFile(manifestPath)
      const response = await testNode.ax.apps.validate(manifestPath)
      expect(response).toMatchErrInvalidInput()
      await remove(manifestPath)
    })
  })

  describe('package', () => {
    const tarballFileName = 'com.actyx.sample-webview-app-1.0.0.tar.gz'
    const tarballFile = path.resolve(workingDir, tarballFileName)
    const regexTarballFile = new RegExp(`${tarballFileName}+$`, 'g')

    const removeTarball = () => remove(tarballFile)

    beforeEach(() => removeTarball())

    afterEach(() => removeTarball())

    test('return ERR_INVALID_INPUT if manifest was not found', async () => {
      const response = await testNode.ax.apps.package(invalidPath)
      expect(response).toMatchErrInvalidInput()
    })

    test('return OK and package an app in the specified directory with manifest', async () => {
      const manifestPath = `${sampleWebviewAppDir}/ax-manifest.yml`
      const response = await testNode.ax.apps.packageCwd(workingDir, manifestPath)
      const responseShape = {
        code: 'OK',
        result: [
          {
            appId: 'com.actyx.sample-webview-app',
            appVersion: '1.0.0',
            packagePath: expect.stringMatching(regexTarballFile),
          },
        ],
      }
      expect(response).toMatchObject(responseShape)

      const wasTarballCreated = await pathExists(tarballFile)
      expect(wasTarballCreated).toBe(true)
    })
  })

  describe('deploy', () => {
    let appId = ''
    let packagePath = ''

    beforeAll(async () => {
      const infoSampleDockerApp = await createPackageSampleDockerApp(testNode)
      appId = infoSampleDockerApp.appId
      packagePath = infoSampleDockerApp.packagePath
    })

    afterAll(async () => remove(packagePath))

    afterEach(async () => {
      await testNode.ax.apps.undeploy(appId)
    })

    test('return ERR_INVALID_INPUT if path does not exist', async () => {
      const response = await testNode.ax.apps.deploy(invalidPath)
      expect(response).toMatchErrInvalidInput()
    })

    test('return ERR_NODE_UNREACHABLE if node is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.apps.deploy(packagePath)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.apps.deploy(packagePath)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK if app can be deployed', async () => {
      const responseLsBeforeDeploy = await testNode.ax.apps.ls()
      const responseLsBeforeDeployShape = { code: 'OK', result: [] }
      expect(responseLsBeforeDeploy).toEqual(responseLsBeforeDeployShape)

      const responseDeploy = await testNode.ax.apps.deploy(packagePath)
      const responseDeployShape = { code: 'OK', result: { redeployed: false } }
      expect(responseDeploy).toEqual(responseDeployShape)

      const responseLsAfterDeploy = await testNode.ax.apps.ls()
      const responseLsAfterDeployShape = {
        code: 'OK',
        result: [
          {
            nodeId: 'localhost',
            appId: 'com.actyx.sample-docker-app',
            version: '1.0.0',
            running: false,
            startedIso: null,
            startedUnix: null,
            licensed: true,
            settingsValid: true,
            enabled: false,
          },
        ],
      }
      expect(responseLsAfterDeploy).toEqual(responseLsAfterDeployShape)
    })

    test('return ERR_APP_ALREADY_DEPLOYED if an app cannot be redeployed', async () => {
      const responseFirstDeploy = await testNode.ax.apps.deploy(packagePath)
      const responseFirstDeployShape = { code: 'OK', result: { redeployed: false } }
      expect(responseFirstDeploy).toEqual(responseFirstDeployShape)

      const responseLsAfterFirstDeploy = await testNode.ax.apps.ls()
      const responseLsAfterFirstDeployShape = {
        code: 'OK',
        result: [
          {
            nodeId: 'localhost',
            appId: 'com.actyx.sample-docker-app',
            version: '1.0.0',
            running: false,
            startedIso: null,
            startedUnix: null,
            licensed: true,
            settingsValid: true,
            enabled: false,
          },
        ],
      }
      expect(responseLsAfterFirstDeploy).toEqual(responseLsAfterFirstDeployShape)

      const responseSecondDeploy = await testNode.ax.apps.deploy(packagePath)
      const responseSecondDeployShape = {
        code: 'ERR_APP_ALREADY_DEPLOYED',
        message: expect.any(String),
      }
      expect(responseSecondDeploy).toEqual(responseSecondDeployShape)

      const responseLsAfterSecondDeploy = await testNode.ax.apps.ls()
      expect(responseLsAfterSecondDeploy).toEqual(responseLsAfterFirstDeployShape)
    })

    test('return OK and force update even if version number has not changed', async () => {
      const responseFirstDeploy = await testNode.ax.apps.deploy(packagePath)
      const responseFirstDeployShape = { code: 'OK', result: { redeployed: false } }
      expect(responseFirstDeploy).toEqual(responseFirstDeployShape)

      const responseSecondDeploy = await testNode.ax.apps.deploy(packagePath, true)
      const responseSecondDeployShape = { code: 'OK', result: { redeployed: true } }
      expect(responseSecondDeploy).toEqual(responseSecondDeployShape)
    })
  })

  describe('undeploy', () => {
    let appId = ''
    let packagePath = ''

    beforeAll(async () => {
      const infoSampleDockerApp = await createPackageSampleDockerApp(testNode)
      appId = infoSampleDockerApp.appId
      packagePath = infoSampleDockerApp.packagePath
    })

    afterAll(async () => remove(packagePath))

    test('return ERR_NODE_UNREACHABLE if host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.apps.undeploy(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.apps.undeploy(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_INVALID_INPUT if it cannot undeploy a not existing app', async () => {
      const responseUndeploy = await testNode.ax.apps.undeploy(invalidPath)
      expect(responseUndeploy).toMatchErrInvalidInput()
    })

    test('return OK if it can undeploy a deployed app', async () => {
      await testNode.ax.apps.deploy(packagePath)
      const responseLsAfterDeploy = await testNode.ax.apps.ls()
      const responseLsAfterDeployShape = {
        code: 'OK',
        result: [
          {
            nodeId: 'localhost',
            appId: 'com.actyx.sample-docker-app',
            version: '1.0.0',
            running: false,
            startedIso: null,
            startedUnix: null,
            licensed: true,
            settingsValid: true,
            enabled: false,
          },
        ],
      }
      expect(responseLsAfterDeploy).toEqual(responseLsAfterDeployShape)

      const responseUndeploy = await testNode.ax.apps.undeploy(appId)
      const responseUndeployShape = {
        code: 'OK',
        result: { appId: 'com.actyx.sample-docker-app', host: 'localhost' },
      }
      expect(responseUndeploy).toEqual(responseUndeployShape)

      const responseLsAfterUndeploy = await testNode.ax.apps.ls()
      const responseLsAfterUndeployShape = { code: 'OK', result: [] }
      expect(responseLsAfterUndeploy).toEqual(responseLsAfterUndeployShape)
    })

    test('return ERR_APP_ENABLED if unable to deploy app since it is currently enabled', async () => {
      await testNode.ax.apps.deploy(packagePath)

      await testNode.ax.apps.start(appId)
      await waitForAppToStart(appId, testNode)

      const responseUndeploy = await testNode.ax.apps.undeploy(appId)
      const responseUndeployShape = { code: 'ERR_APP_ENABLED', message: expect.any(String) }
      expect(responseUndeploy).toMatchObject(responseUndeployShape)

      await testNode.ax.apps.stop(appId)
      await waitForAppToStop(appId, testNode)
    })
  })

  describe('start', () => {
    let appId = ''
    let packagePath = ''

    beforeAll(async () => {
      const infoSampleDockerApp = await createPackageSampleDockerApp(testNode)
      appId = infoSampleDockerApp.appId
      packagePath = infoSampleDockerApp.packagePath
    })

    afterAll(async () => remove(packagePath))

    afterEach(async () => {
      await testNode.ax.apps.undeploy(appId)
    })

    test('return ERR_NODE_UNREACHABLE if host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.apps.start(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.apps.start(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with alreadyStarted set if ax tries to start an already started app', async () => {
      await testNode.ax.apps.deploy(packagePath)
      const responseStart = await testNode.ax.apps.start(appId)
      const responseStartShape = {
        code: 'OK',
        result: { appId: 'com.actyx.sample-docker-app', host: 'localhost', alreadyStarted: false },
      }
      expect(responseStart).toEqual(responseStartShape)

      await waitFor(async () => {
        const responseLs = await testNode.ax.apps.ls()
        const responseLsShape = {
          code: 'OK',
          result: [
            {
              nodeId: 'localhost',
              appId: 'com.actyx.sample-docker-app',
              version: '1.0.0',
              running: true,
              startedIso: expect.any(String),
              startedUnix: expect.any(Number),
              licensed: true,
              settingsValid: true,
              enabled: true,
            },
          ],
        }
        expect(responseLs).toMatchObject(responseLsShape)
      })

      const responseStartAgain = await testNode.ax.apps.start(appId)
      const responseStartAgainShape = {
        code: 'OK',
        result: { appId: 'com.actyx.sample-docker-app', host: 'localhost', alreadyStarted: true },
      }
      expect(responseStartAgain).toEqual(responseStartAgainShape)

      await testNode.ax.apps.stop(appId)
      await waitForAppToStop(appId, testNode)
    })
  })

  describe('stop', () => {
    let appId = ''
    let packagePath = ''

    beforeAll(async () => {
      const infoSampleDockerApp = await createPackageSampleDockerApp(testNode)
      appId = infoSampleDockerApp.appId
      packagePath = infoSampleDockerApp.packagePath
    })

    afterAll(async () => remove(packagePath))

    afterEach(async () => {
      await testNode.ax.apps.undeploy(appId)
    })

    test('return ERR_NODE_UNREACHABLE if host is unreachable', async () => {
      const response = await stubs.hostUnreachable.ax.apps.stop(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubs.actyxOSUnreachable.ax.apps.stop(appId)
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with alreadyStopped if we try to stop an already stopped app', async () => {
      await testNode.ax.apps.deploy(packagePath)

      await testNode.ax.apps.start(appId)
      await waitForAppToStart(appId, testNode)

      const responseStopFirst = await testNode.ax.apps.stop(appId)
      const responseStopFirstShape = {
        code: 'OK',
        result: { appId: 'com.actyx.sample-docker-app', host: 'localhost', alreadyStopped: false },
      }
      expect(responseStopFirst).toEqual(responseStopFirstShape)

      const responseStopSecond = await testNode.ax.apps.stop(appId)
      const responseStopSecondShape = {
        code: 'OK',
        result: { appId: 'com.actyx.sample-docker-app', host: 'localhost', alreadyStopped: true },
      }
      expect(responseStopSecond).toEqual(responseStopSecondShape)
    })
  })
})
