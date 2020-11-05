import { runOnAll, runOnEach } from '../runner/hosts'
import { stubNode, stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import { isCodeInvalidInput, isCodeNodeUnreachable, isCodeOk } from './util'
import { remove, pathExists } from 'fs-extra'
import { Response_Apps_Package } from './types'
import testProjects from './setup-projects/test-projects'

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

    test('return empty result if no apps', async () => {
      const responses = await stubNode.ax.Apps.Ls()
      const test = { code: 'OK', result: [] }
      expect(responses).toMatchObject(test)
    })
  })
  describe('validate', () => {
    test('return `ERR_INVALID_INPUT` if file path does not exist', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.Validate('not-existing-path')
      expect(isCodeInvalidInput(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with default manifest', async () => {
      const manifestPath = testProjects.quickstart.dirs.dirSampleWebviewApp
      const manifestDefault = 'temp/quickstart/sample-webview-app'
      const response = await stubNode.ax.Apps.Validate(manifestPath)
      const reponseShape = { code: 'OK', result: [manifestDefault] }
      expect(response).toMatchObject(reponseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate with default manifest', async () => {
      const cwdDir = testProjects.quickstart.dirs.dirSampleWebviewApp
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.ValidateCwd(cwdDir))
      const reponseShape = { code: 'OK', result: ['ax-manifest.yml'] }
      expect(response).toMatchObject(reponseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${testProjects.quickstart.dirs.dirSampleWebviewApp}/ax-manifest.yml`
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.Validate(manifestPath))
      const reponseShape = { code: 'OK', result: [manifestPath] }
      expect(response).toMatchObject(reponseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return multiple `ERR_INVALID_INPUT` if input paths do not exist for multiple apps', async () => {
      const [response1, response2] = await runOnAll([{}, {}], false, ([node1, node2]) =>
        Promise.all([
          node1.ax.Apps.Validate('not-existing-path1'),
          node2.ax.Apps.Validate('not-existing-path2'),
        ]),
      )
      expect(isCodeInvalidInput(response1)).toBe(true)
      expect(isCodeInvalidInput(response2)).toBe(true)
    })

    test('return multiple `OK` an validate apps if input paths do exists for multiple apps', async () => {
      const { dirDashboard, dirErpSimulator } = testProjects.demoMachineKit.dirs
      const [response1, response2] = await runOnAll([{}, {}], false, ([node1, node2]) =>
        Promise.all([
          node1.ax.Apps.Validate(dirDashboard),
          node2.ax.Apps.Validate(dirErpSimulator),
        ]),
      )
      const reponse1Shape = { code: 'OK', result: ['temp/DemoMachineKit/src/dashboard'] }
      const reponse2Shape = { code: 'OK', result: ['temp/DemoMachineKit/src/erp-simulator'] }
      expect(response1).toMatchObject(reponse1Shape)
      expect(response2).toMatchObject(reponse2Shape)
      expect(isCodeOk(response1)).toBe(true)
      expect(isCodeOk(response2)).toBe(true)
    })
  })

  describe('package', () => {
    const tarballFile = 'com.actyx.sample-webview-app-1.0.0.tar.gz'

    const haveValidPacakgePath = (response: Response_Apps_Package, tarballFile: string) =>
      response.code === 'OK' && response.result.every((x) => x.packagePath.endsWith(tarballFile))

    test('return `ERR_INVALID_INPUT` if manifest was not found', async () => {
      const [reponse] = await runOnEach([{}], false, (node) =>
        node.ax.Apps.Package('not-exiting-path'),
      )
      expect(isCodeInvalidInput(reponse)).toBe(true)
    })

    test('return `OK` and Package an app in the current directory with default manifest ax-manifest.yml', async () => {
      await remove(tarballFile)

      const [response] = await runOnEach([{}], false, (node) =>
        node.ax.Apps.PackageCwd(testProjects.quickstart.dirs.dirSampleWebviewApp),
      )

      expect(isCodeOk(response)).toBe(true)
      expect(haveValidPacakgePath(response, tarballFile)).toBe(true)

      await remove(tarballFile)
    })

    test('return `OK` and package an app in the specified directory with manifest', async () => {
      await remove(tarballFile)

      const manifestPath = `${testProjects.quickstart.dirs.dirSampleWebviewApp}/ax-manifest.yml`
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.Package(manifestPath))

      expect(isCodeOk(response)).toBe(true)
      expect(haveValidPacakgePath(response, tarballFile)).toBe(true)

      const wasTarballCreated = await pathExists(tarballFile)
      expect(wasTarballCreated).toBe(true)

      await remove(tarballFile)
    })

    /* Test cases:

    DONE 
    # Package an app in the current directory with default manifest ax-manifest.yml
    ax apps package

    TODO
    # Package an app in the current directory with manifest another-manifest.yml
    ax apps package another-manifest.yml

    TODO
    # Package the app in the specified directory myApp with default manifest
    # ax-manifest.yml
    ax apps package myApp/

    DONE
    # Package an app in the specified directory myApp with manifest
    # myApp/another-manifest.yml
    ax apps package myApp/another-manifest.yml
    */
  })
})
