import { REQUEST_OPTIONS_SUBSCRIBE_MONOTONIC } from '../../httpClient'
import { getEventsInStreamAfterMs } from '../../stream'
import { getNodeId, mkStreamId, publishEvent, randomString } from '../../util'
import { EventDelivered } from '../../event-service-types'

const TIMEOUT_MS = 3000

describe('event service', () => {
  describe.skip('subscribe to event streams monotonically', () => {
    it('sould publish event and find it in a monotonic event stream', async () => {
      const FIND_VALUE = randomString()

      const nodeId = await getNodeId()
      await publishEvent(FIND_VALUE)
      const streamId = mkStreamId(nodeId)
      const postData = JSON.stringify({
        session: '<my_session_id>',
        offsets: { [streamId]: 1000 },
        where: "'integration' & 'test:1'",
      })

      await getEventsInStreamAfterMs<EventDelivered>(
        REQUEST_OPTIONS_SUBSCRIBE_MONOTONIC,
        TIMEOUT_MS,
        postData,
      ).then((events) => {
        expect(events[0]).toMatchObject({
          type: 'event',
          lamport: expect.any(Number),
          stream: expect.any(String),
          offset: expect.any(Number),
          timestamp: expect.any(Number),
          tags: ['integration', 'test:1'],
          payload: {
            value: expect.any(String),
          },
          caughtUp: expect.any(Boolean),
        })

        expect(events.find((x) => x.payload.value === FIND_VALUE)?.payload.value).toBe(FIND_VALUE)
      })
    })
  })
})
