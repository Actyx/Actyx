import { runOnEach } from '../runner/hosts'
import { fakeNodeActyxosUnreachable, fakeNodeHostUnreachable } from '../util'
import { isCodeInvalidInput, isCodeNodeUnreachable } from './util'

describe('ax apps', () => {
  describe('ls', () => {
    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await fakeNodeHostUnreachable.ax.Apps.Ls()
      expect(isCodeNodeUnreachable(r)).toBe(true)
    })

    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await fakeNodeActyxosUnreachable.ax.Apps.Ls()
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

    describe('validate', () => {
      test('return `ERR_INVALID_INPUT` if path does not exist', async () => {
        const response = await fakeNodeHostUnreachable.ax.Apps.Validate('foo')
        expect(isCodeInvalidInput(response)).toBe(true)
      })
    })

    //TODO: test it all
  })
})
