import { REQUEST_OPTIONS_SUBSCRIBE } from '../../httpClient'
import { findEventsInStream } from '../../stream'
import { getNodeId, mkStreamId, publishEvent, randomString } from '../../util'

const TIMEOUT_MS = 3000

describe.skip('event service', () => {
  describe('subscribe to event streams', () => {
    it('should publish event and find it an event stream', async () => {
      const FIND_VALUE = randomString()

      const nodeId = await getNodeId()
      await publishEvent(FIND_VALUE)
      const streamId = mkStreamId(nodeId)
      const postData = JSON.stringify({
        offsets: { [streamId]: 0 },
        where: "'integration' & 'test:1'",
      })

      await findEventsInStream(REQUEST_OPTIONS_SUBSCRIBE, TIMEOUT_MS, FIND_VALUE, postData)
    })
  })
})
