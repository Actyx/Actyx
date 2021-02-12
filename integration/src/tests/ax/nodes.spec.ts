import { assertOK } from '../../assertOK'
import { runOnEvery } from '../../infrastructure/hosts'
import { stubs } from '../../stubs'

describe('ax nodes', () => {
  describe('ls', () => {
    test('return Ok and result with connection hostUnreachable', async () => {
      const response = await stubs.unreachable.ax.nodes.ls()
      expect(response).toMatchObject({
        code: 'OK',
        result: [
          {
            connection: 'unreachable',
            host: expect.any(String),
          },
        ],
      })
    })

    test('return OK and result with connection reachable', async () => {
      await runOnEvery({}, async (node) => {
        const response = assertOK(await node.ax.nodes.ls())
        const responseShape = [
          {
            connection: 'reachable',
            host: expect.any(String),
            nodeId: expect.any(String),
            displayName: node.name,
            startedIso: expect.any(String),
            startedUnix: expect.any(Number),
            version: '2.0.0-dev',
          },
        ]
        expect(response.result).toMatchObject(responseShape)
      })
    })
  })
})
