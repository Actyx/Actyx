import { stubNode, stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import { remove, pathExists } from 'fs-extra'
import { quickstartDirs } from './setup-projects/quickstart'
import { demoMachineKitDirs } from './setup-projects/demo-machine-kit'

describe('ax apps', () => {
  describe('ls', () => {
    test('return ERR_NODE_UNREACHABLE if host is unreachable', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.Ls()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return ERR_NODE_UNREACHABLE if actyxos is unreachable', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Apps.Ls()
      expect(response).toMatchErrNodeUnreachable()
    })

    test('return OK with empty result if no apps are installed', async () => {
      const responses = await stubNode.ax.Apps.Ls()
      const responseShape = { code: 'OK', result: [] }
      expect(responses).toMatchObject(responseShape)
    })
  })

  describe('validate', () => {
    test('return ERR_INVALID_INPUT if file path does not exist', async () => {
      const response = await stubNode.ax.Apps.Validate('not-existing-path')
      expect(response).toMatchErrInvalidInput()
    })

    test('return OK and validate an app in the specified directory with default manifest', async () => {
      const manifestPath = quickstartDirs.sampleWebviewApp
      const manifestDefault = 'temp/quickstart/sample-webview-app'
      const response = await stubNode.ax.Apps.Validate(manifestPath)
      const responseShape = { code: 'OK', result: [manifestDefault] }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and validate an app with default manifest', async () => {
      const cwdDir = quickstartDirs.sampleWebviewApp
      const response = await stubNode.ax.Apps.ValidateCwd(cwdDir)
      const responseShape = { code: 'OK', result: ['ax-manifest.yml'] }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstartDirs.sampleWebviewApp}/ax-manifest.yml`
      const response = await stubNode.ax.Apps.Validate(manifestPath)
      const responseShape = { code: 'OK', result: [manifestPath] }
      expect(response).toMatchObject(responseShape)
    })

    test('return ERR_INVALID_INPUT if input paths do not exist for multiple apps', async () => {
      const response = await stubNode.ax.Apps.ValidateMultiApps([
        'not-existing-path1',
        'not-existing-path2',
      ])
      expect(response).toMatchErrInvalidInput()
    })

    test('return OK and validate apps if input paths do exists', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.ValidateMultiApps([
        demoMachineKitDirs.dashboard,
        demoMachineKitDirs.erpSimulator,
      ])
      const responseShape = {
        code: 'OK',
        result: ['temp/DemoMachineKit/src/dashboard', 'temp/DemoMachineKit/src/erp-simulator'],
      }
      expect(response).toMatchObject(responseShape)
    })
  })

  describe('package', () => {
    const tarballFile = 'com.actyx.sample-webview-app-1.0.0.tar.gz'
    const regexTarballFile = new RegExp(`${tarballFile}+$`, 'g')

    const removeTarballs = async () => {
      await remove(`${tarballFile}`)
      await remove(`${quickstartDirs.sampleWebviewApp}/${tarballFile}`)
    }

    beforeEach(() => removeTarballs())

    afterEach(() => removeTarballs())

    test('return ERR_INVALID_INPUT if manifest was not found', async () => {
      const response = await stubNode.ax.Apps.Package('not-exiting-path')
      expect(response).toMatchErrInvalidInput()
    })

    test('return OK and package an app in the current directory with default manifest ax-manifest.yml', async () => {
      const response = await stubNode.ax.Apps.PackageCwd(quickstartDirs.sampleWebviewApp)
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
    })

    test('return OK and package an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstartDirs.sampleWebviewApp}/ax-manifest.yml`
      const response = await stubNode.ax.Apps.Package(manifestPath)
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
})
