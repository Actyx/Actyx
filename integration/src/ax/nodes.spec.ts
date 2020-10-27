import { runOnEach } from '../runner/hosts'
import { fakeNodeHostUnreachable, fakeNodeActyxosUnreachable } from '../util'
import { Response_Nodes_Ls } from './types'

const areConnectionsOfStatus = (status: string) => (r: Response_Nodes_Ls) =>
  r.code === 'OK' && r.result.every((r) => r.connection === status)

const areReachable = areConnectionsOfStatus('reachable')
const areHostUnreachable = areConnectionsOfStatus('hostUnreachable')
const areActyxosUnreachable = areConnectionsOfStatus('actyxosUnreachable')

describe('ax nodes', () => {
  describe('ls', () => {
    test('return connection `hostUnreachable`', async () => {
      const response = await fakeNodeHostUnreachable.ax.Nodes.Ls()
      expect(areHostUnreachable(response)).toBe(true)
    })

    test('return connection `actyxosUnreachable`', async () => {
      const response = await fakeNodeActyxosUnreachable.ax.Nodes.Ls()
      expect(areActyxosUnreachable(response)).toBe(true)
    })

    test('return connection `reachable`', async () => {
      const responses = await runOnEach([{}, {}], false, (node) => node.ax.Nodes.Ls())
      const areValid = responses.every(areReachable)
      expect(areValid).toBe(true)
    })
  })
})
