/**
 * @jest-environment ./dist/integration/src/jest/environment
 */
import { assertOK } from '../assertOK'
import { mkNodeIdService as mkNodeService, mkTrialHttpClient } from '../http-client'
import { runOnEvery } from '../infrastructure/hosts'
import { getHttpApi } from '../util'

describe('node service', () => {
  describe('node/id', () => {
    it('should return node id', () =>
      runOnEvery(async (node) => {
        const nodeInfo = assertOK(await node.ax.nodes.ls()).result[0]
        const nodeId =
          nodeInfo.connection === 'reachable' ? nodeInfo.nodeId : fail('node not reachable')
        const result = await mkTrialHttpClient(getHttpApi(node)).then((x) =>
          mkNodeService(x).nodeId(),
        )
        expect(result).toBe(nodeId)
      }))
  })
})
