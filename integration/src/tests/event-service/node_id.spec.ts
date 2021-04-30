import { mkEventService, mkTrialHttpClient } from '../../http-client'
import { run } from '../../util'

describe('event service', () => {
  describe('node_id', () => {
    it('should return node id', () =>
      run((x) =>
        mkTrialHttpClient(x)
          .then((x) => mkEventService(x).nodeId())
          .then((x) => {
            expect(x.nodeId).toHaveLength(43)
          }),
      ))
  })
})
