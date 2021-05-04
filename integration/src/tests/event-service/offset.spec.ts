import { mkEventService, mkTrialHttpClient } from '../../http-client'
import { run } from '../../util'

describe('event service', () => {
  describe('offsets', () => {
    it('get', () =>
      run(async (x) => {
        const es = mkEventService(await mkTrialHttpClient(x))
        const { nodeId } = await es.nodeId()
        const { present } = await es.offsets()
        const streamId = Object.keys(present).find((x) => x.startsWith(nodeId))
        expect(streamId).toEqual(expect.any(String))
        expect(present[streamId || '']).toEqual(expect.any(Number))
      }))
  })
})
