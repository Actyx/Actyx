/**
 * @jest-environment ./dist/integration/src/jest/environment
 */
import { Actyx, NodeStatus } from '@actyx/sdk'
import { assertOK } from '../assertOK'
import { mkNodeIdService as mkNodeService, mkTrialHttpClient, trialManifest } from '../http-client'
import { runOnEvery } from '../infrastructure/hosts'
import { retryTimes } from '../retry'
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
  describe('node/info', () => {
    it('should return node info', () =>
      runOnEvery(async (node) => {
        const sdk = await Actyx.of(trialManifest, {
          actyxHost: node._private.hostname,
          actyxPort: node._private.apiPort,
        })
        const info = await sdk.nodeInfo(0)
        expect(info.longVersion()).toMatch(process.env.ACTYX_VERSION || 'ACTYX_VERSION not set')
        await retryTimes(async () => {
          // takes two gossip intervals to get first data
          const peers = (await sdk.nodeInfo(0)).peersStatus()
          expect(peers).toMatchObject({ [sdk.nodeId]: NodeStatus.LowLatency })
        }, 30)
      }))
  })
})
