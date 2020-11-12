import { stubNodeHostUnreachable, stubNodeActyxosUnreachable, stubNode } from '../../stubs'

describe('ax nodes', () => {
  describe('ls', () => {
    test('return OK and result with connection hostUnreachable', async () => {
      const response = await stubNodeHostUnreachable.ax.Nodes.Ls()
      const responseShape = {
        code: 'OK',
        result: [{ connection: 'hostUnreachable', host: '123' }],
      }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and result with connection actyxosUnreachable', async () => {
      const response = await stubNodeActyxosUnreachable.ax.Nodes.Ls()
      const responseShape = {
        code: 'OK',
        result: [{ connection: 'actyxosUnreachable', host: 'localhost' }],
      }
      expect(response).toMatchObject(responseShape)
    })

    test('return OK and result with connection reachable', async () => {
      const response = await stubNode.ax.Nodes.Ls()
      const responseShape = {
        code: 'OK',
        result: [
          {
            connection: 'reachable',
            nodeId: 'localhost',
            displayName: null,
            state: 'running',
            settingsValid: false,
            licensed: false,
            appsDeployed: 0,
            appsRunning: 0,
            startedIso: expect.any(String),
            startedUnix: expect.any(Number),
            version: '1.0.0',
          },
        ],
      }
      expect(response).toMatchObject(responseShape)
    })
  })
})
