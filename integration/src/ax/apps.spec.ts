import { runOnEach } from '../runner/hosts'
import { stubNodeActyxosUnreachable, stubNodeHostUnreachable } from '../stubs'
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

    test('return `OK` using default path', async () => {
      const response = await stubNodeHostUnreachable.ax.Apps.Validate(
        quickstart.dirSampleWebviewApp,
      )
      const reponseShape = { code: 'OK', result: ['temp/quickstart/sample-webview-app'] }
      expect(isCodeOk(response)).toBe(true)
      expect(response).toMatchObject(reponseShape)
    })
  })
})
