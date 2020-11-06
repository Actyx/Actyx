import { stubNode, stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import { isCodeInvalidInput, isCodeNodeUnreachable, isCodeOk } from './util'
import { remove, pathExists } from 'fs-extra'
import testProjects from './setup-projects'

const { demoMachineKit, quickstart } = testProjects

describe('ax apps', () => {
  describe('ls', () => {
    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await stubNodeHostUnreachable.ax.Apps.Ls()
      expect(isCodeNodeUnreachable(r)).toBe(true)
    })

    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await stubNodeActyxosUnreachable.ax.Apps.Ls()
      expect(isCodeNodeUnreachable(r)).toBe(true)
    })

    test('return `OK` and empty result if no apps', async () => {
      const responses = await stubNode.ax.Apps.Ls()
      const test = { code: 'OK', result: [] }
      expect(responses).toMatchObject(test)
    })
  })

  describe('validate', () => {
    test('return `ERR_INVALID_INPUT` if file path does not exist', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.Validate('not-existing-path')
      expect(response.code === 'ERR_INVALID_INPUT' && response).toHaveProperty('message')
      expect(isCodeInvalidInput(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with default manifest', async () => {
      const manifestPath = quickstart.dirs.dirSampleWebviewApp
      const manifestDefault = 'temp/quickstart/sample-webview-app'
      const response = await stubNode.ax.Apps.Validate(manifestPath)
      const responseShape = { code: 'OK', result: [manifestDefault] }
      expect(response).toMatchObject(responseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate with default manifest', async () => {
      const cwdDir = quickstart.dirs.dirSampleWebviewApp
      const response = await stubNode.ax.Apps.ValidateCwd(cwdDir)
      const responseShape = { code: 'OK', result: ['ax-manifest.yml'] }
      expect(response).toMatchObject(responseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstart.dirs.dirSampleWebviewApp}/ax-manifest.yml`
      const response = await stubNode.ax.Apps.Validate(manifestPath)
      const responseShape = { code: 'OK', result: [manifestPath] }
      expect(response).toMatchObject(responseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return multiple `ERR_INVALID_INPUT` if input paths do not exist for multiple apps', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.ValidateMultiApps([
        'not-existing-path1',
        'not-existing-path2',
      ])
      expect(response.code === 'ERR_INVALID_INPUT' && response).toHaveProperty('message')
      expect(isCodeInvalidInput(response)).toBe(true)
    })

    test('return multiple `OK` an validate apps if input paths do exists for multiple apps', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.ValidateMultiApps([
        demoMachineKit.dirs.dirDashboard,
        demoMachineKit.dirs.dirErpSimulator,
      ])
      const responseShape = {
        code: 'OK',
        result: ['temp/DemoMachineKit/src/dashboard', 'temp/DemoMachineKit/src/erp-simulator'],
      }
      expect(response).toMatchObject(responseShape)
      expect(isCodeOk(response)).toBe(true)
    })
  })

  describe('package', () => {
    const tarballFile = 'com.actyx.sample-webview-app-1.0.0.tar.gz'
    const regexTarballFile = new RegExp(`${tarballFile}+$`, 'g')

    const removeTarballs = async () => {
      await remove(`${tarballFile}`)
      await remove(`${quickstart.dirs.dirSampleWebviewApp}/${tarballFile}`)
    }

    beforeEach(() => removeTarballs())
    afterEach(() => removeTarballs())

    test('return `ERR_INVALID_INPUT` if manifest was not found', async () => {
      const response = await stubNode.ax.Apps.Package('not-exiting-path')
      expect(response.code === 'ERR_INVALID_INPUT' && response).toHaveProperty('message')
      expect(isCodeInvalidInput(response)).toBe(true)
    })

    test('return `OK` and Package an app in the current directory with default manifest ax-manifest.yml', async () => {
      const response = await stubNode.ax.Apps.PackageCwd(quickstart.dirs.dirSampleWebviewApp)

      expect(isCodeOk(response)).toBe(true)
      expect(response.code === 'OK' && response.result[0]).toHaveProperty('packagePath')
      expect(response.code === 'OK' && response.result[0].packagePath).toMatch(regexTarballFile)
    })

    test('return `OK` and package an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstart.dirs.dirSampleWebviewApp}/ax-manifest.yml`
      const response = await stubNode.ax.Apps.Package(manifestPath)

      expect(isCodeOk(response)).toBe(true)
      expect(response.code === 'OK' && response.result[0]).toHaveProperty('packagePath')
      expect(response.code === 'OK' && response.result[0].packagePath).toMatch(regexTarballFile)

      const wasTarballCreated = await pathExists(tarballFile)
      expect(wasTarballCreated).toBe(true)
    })
  })
})
