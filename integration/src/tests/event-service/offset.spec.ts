import { OffsetsResponse } from '../../event-service-types'
import { httpClient } from '../../httpClient'
import { getNodeId, mkStreamId } from '../../util'

describe.skip('event service', () => {
  describe('get information about known offsets', () => {
    it('should return valid result for offset', async () => {
      const nodeId = await getNodeId()
      const offsetsRes = await httpClient.get<OffsetsResponse>('offsets')
      const streamId = mkStreamId(nodeId)
      const hasSomeStreamIdsWhichStartWithNodeId = Object.keys(offsetsRes.data).some((x) =>
        x.startsWith(streamId),
      )
      expect(offsetsRes.status).toBe(200)
      expect(typeof offsetsRes.data[streamId]).toBe('number')
      expect(hasSomeStreamIdsWhichStartWithNodeId).toBeTruthy()
    })
  })
})
