import { assertOK } from '../assertOK'
import { mkNodeIdService as mkNodeService, mkTrialHttpClient } from '../http-client'
import { runOnEvery } from '../infrastructure/hosts'

describe('node service', () => {
  describe('node_id', () => {
    it('should return node id', () =>
      runOnEvery(async (x) => {
        const nodeInfo = assertOK(await x.ax.nodes.ls()).result[0]
        const nodeId =
          nodeInfo.connection === 'reachable' ? nodeInfo.nodeId : fail('node not reachable')
        const result = await mkTrialHttpClient(x._private.httpApiOrigin).then((x) =>
          mkNodeService(x).nodeId(),
        )
        expect(result.nodeId).toBe(nodeId)
      }))
  })
})
