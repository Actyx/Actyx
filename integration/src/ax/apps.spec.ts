import { runOnAll, runOnEach } from '../runner/hosts'
import { stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
import demoMachineKit from './setup-projects/demo-machine-kit'
import quickstart from './setup-projects/quickstart'
import { isCodeInvalidInput, isCodeNodeUnreachable, isCodeOk } from './util'

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
      // TODO add control for cwd tp Apps.Validate so I can test properly current directory behaviour
      const manifestPath = quickstart.dirSampleWebviewApp
      const manifestDefault = 'temp/quickstart/sample-webview-app'
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.Validate(manifestPath))
      const reponseShape = { code: 'OK', result: [manifestDefault] }
      expect(isCodeOk(response)).toBe(true)
      expect(response).toMatchObject(reponseShape)
    })

    test('return `OK` and validate an app in the specified directory with manifest', async () => {
      const manifestPath = `${quickstart.dirSampleWebviewApp}/ax-manifest.yml`
      const [response] = await runOnEach([{}], false, (node) => node.ax.Apps.Validate(manifestPath))
      const reponseShape = { code: 'OK', result: [manifestPath] }
      expect(isCodeOk(response)).toBe(true)
      expect(response).toMatchObject(reponseShape)
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
      expect(isCodeOk(response1)).toBe(true)
      expect(isCodeOk(response2)).toBe(true)
      expect(response1).toMatchObject(reponse1Shape)
      expect(response2).toMatchObject(reponse2Shape)
    })

    // TODO add vadalite app in current directory with default manifest
  })
})
