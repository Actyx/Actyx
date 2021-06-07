import { mkEventService, mkNodeIdService, mkTrialHttpClient } from '../../http-client'
import { run } from '../../util'

describe('event service', () => {
  describe('offsets', () => {
    it('get', () =>
      run(async (x) => {
        const ns = mkNodeIdService(await mkTrialHttpClient(x))
        const nodeId = await ns.nodeId()
        const es = mkEventService(await mkTrialHttpClient(x))
        const { present } = await es.offsets()
        // stream 1 is for discovery events, which is the only stream guaranteed to have events from the start
        // (there are at least two addresses bound: primary interface and localhost, so at least two events)
        expect(present[`${nodeId}-1`]).toBeGreaterThan(0)
      }))
  })
})
