import { runOnEach } from '../runner/hosts'
import { testNodeActyxosUnreachable, testNodeHostUnreachable } from '../util'
import { isCodeNodeUnreachable } from './util'

describe('ax apps', () => {
  describe('ls', () => {
    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await testNodeHostUnreachable.ax.Apps.Ls()
      expect(isCodeNodeUnreachable(r)).toBe(true)
    })

    test('return `ERR_NODE_UNREACHABLE`', async () => {
      const r = await testNodeActyxosUnreachable.ax.Apps.Ls()
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

    //TODO: test it all
  })
})
