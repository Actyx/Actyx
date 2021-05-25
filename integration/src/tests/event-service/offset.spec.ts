import { mkEventService, mkTrialHttpClient } from '../../http-client'
import { run } from '../../util'

describe('event service', () => {
  describe('offsets', () => {
    it('get', () =>
      run(async (x) => {
        const es = mkEventService(await mkTrialHttpClient(x))
        const { nodeId } = await es.nodeId()
        const { present } = await es.offsets()
        // stream 1 is for discovery events, which is the only stream guaranteed to have events from the start
        // (there are at least two addresses bound: primary interface and localhost, so at least two events)
        console.log(present, nodeId)
        expect(present[`${nodeId}-1`]).toBeGreaterThan(0)
      }))
  })
})
