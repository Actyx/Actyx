import { assertOK } from '../../assertOK'
import { runOnEvery } from '../../infrastructure/hosts'
import { stubNodeHostUnreachable, stubNodeActyxosUnreachable } from '../../stubs'

describe('ax nodes', () => {
  describe('ls', () => {
    test('return OK and result with connection hostUnreachable', async () => {
      const response = assertOK(await stubNodeHostUnreachable.ax.Nodes.Ls())
      expect(response.result).toMatchObject([{ connection: 'hostUnreachable', host: '123' }])
    })

    test('return OK and result with connection actyxosUnreachable', async () => {
      const response = assertOK(await stubNodeActyxosUnreachable.ax.Nodes.Ls())
      expect(response.result).toMatchObject([
        { connection: 'actyxosUnreachable', host: 'localhost' },
      ])
    })

    test('return OK and result with connection reachable', async () => {
      await runOnEvery({}, async (node) => {
        const response = assertOK(await node.ax.Nodes.Ls())
        const responseShape = [
          {
            connection: 'reachable',
            nodeId: 'localhost',
            displayName: node.name,
            state: 'running',
            settingsValid: true,
            licensed: true,
            appsDeployed: expect.any(Number),
            appsRunning: expect.any(Number),
            startedIso: expect.any(String),
            startedUnix: expect.any(Number),
            version: '1.0.0',
          },
        ]
        expect(response.result).toMatchObject(responseShape)
      })
    })
  })
})
