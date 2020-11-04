import { runOnAll, runOnEach } from '../runner/hosts'
import { stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import { demoMachineKit, quickstart } from './setup-projects'
import { isCodeInvalidInput, isCodeNodeUnreachable, isCodeOk } from './util'
import { remove, pathExists } from 'fs-extra'

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
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Apps.Ls())
      const test = [
        { code: 'OK', result: [] },
        { code: 'OK', result: [] },
      ]
      expect(responses).toMatchObject(test)
    })
  })
  describe('validate', () => {
    test('return `ERR_INVALID_INPUT` if file path does not exist', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.Validate('not-existing-path')
      expect(isCodeInvalidInput(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with default manifest', async () => {
      const manifestPath = quickstart.dirSampleWebviewApp
      const manifestDefault = 'temp/quickstart/sample-webview-app'
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.Validate(manifestPath))
      const reponseShape = { code: 'OK', result: [manifestDefault] }
      expect(response).toMatchObject(reponseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate with default manifest', async () => {
      const cwdDir = quickstart.dirSampleWebviewApp
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.ValidateCwd(cwdDir))
      const reponseShape = { code: 'OK', result: ['ax-manifest.yml'] }
      expect(response).toMatchObject(reponseShape)
      expect(isCodeOk(response)).toBe(true)
    })

    test('return `OK` and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstart.dirSampleWebviewApp}/ax-manifest.yml`
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
      const { dirDashboard, dirErpSimulator } = demoMachineKit
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
    test('return `ERR_INVALID_INPUT` if manifest was not found', async () => {
      const [reponse] = await runOnEach([{}], false, (node) =>
        node.ax.Apps.Package('not-exiting-path'),
      )
      expect(isCodeInvalidInput(reponse)).toBe(true)
    })

    test('return `OK` and package app if input path does exist', async () => {
      const tarballFile = 'com.actyx.sample-webview-app-1.0.0.tar.gz'

      await remove(tarballFile)

      const [reponse] = await runOnEach([{}, {}], false, (node) =>
        node.ax.Apps.Package(quickstart.dirSampleWebviewApp),
      )
      const haveValidPacakgePath =
        reponse.code === 'OK' && reponse.result.every((x) => x.packagePath.endsWith(tarballFile))

      expect(isCodeOk(reponse)).toBe(true)
      expect(haveValidPacakgePath).toBe(true)

      const wasTarballCreated = await pathExists(tarballFile)
      expect(wasTarballCreated).toBe(true)

      await remove(tarballFile)
    })
  })
})
