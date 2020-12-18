import { assertOK } from '../../assertOK'
import { runOnEvery } from '../../infrastructure/hosts'
import { stubs } from '../../stubs'

describe('ax nodes', () => {
  describe('ls', () => {
    test('return OK and result with connection hostUnreachable', async () => {
      const response = assertOK(await stubs.hostUnreachable.ax.nodes.ls())
      expect(response.result).toMatchObject([{ connection: 'hostUnreachable', host: 'idontexist' }])
    })

    test('return OK and result with connection actyxosUnreachable', async () => {
      const response = assertOK(await stubs.actyxOSUnreachable.ax.nodes.ls())
      expect(response.result).toMatchObject([
        { connection: 'actyxosUnreachable', host: 'localhost' },
      ])
    })

    test('return OK and result with connection reachable', async () => {
      await runOnEvery({}, async (node) => {
        const response = assertOK(await node.ax.nodes.ls())
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
            version: '1.1.1',
          },
        ]
        expect(response.result).toMatchObject(responseShape)
      })
    })
  })
})
