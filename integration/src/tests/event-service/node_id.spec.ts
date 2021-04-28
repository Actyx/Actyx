import { NodeIdResponse } from '../../event-service-types'
import { httpClient } from '../../httpClient'

describe.skip('event service', () => {
  describe('get node id', () => {
    it('should return node id information', async () => {
      await httpClient.get<NodeIdResponse>('node_id').then((response) => {
        expect(response.status).toBe(200)
        expect(response.data).toMatchObject({
          nodeId: expect.any(String),
        })
        expect(response.data.nodeId).toHaveLength(43)
      })
    })
  })
})
