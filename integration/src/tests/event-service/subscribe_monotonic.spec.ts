import { SubscribeMonotonicRequest, SubscribeMonotonicResponse } from '../../http-client'
import { waitFor } from '../../retry'
import { runWithClients } from '../../util'
import { genericCommunicationTimeout, mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('subscribe_monotonic', () => {
    it('should publish event and find it in a monotonic event stream', () =>
      runWithClients(async (events, clientId) => {
        const pub1 = await publishRandom(events, clientId)
        const request: SubscribeMonotonicRequest = {
          session: 'test-session',
          query: `FROM '${mySuite()}' & '${testName()}' & 'client:${clientId}' & isLocal`,
          lowerBound: {},
        }
        const data: SubscribeMonotonicResponse[] = []
        await new Promise((resolve, reject) => {
          events
            .subscribeMonotonic(request, (x) => {
              data.push(x)
              if (data.length == 2) {
                resolve()
              }
            })
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })
        expect(data).toMatchObject([
          pub1,
          { type: 'offsets', offsets: { [pub1.stream]: expect.any(Number) } },
        ])
      }))

    it('should start a monotonic event stream and find published event', () =>
      runWithClients(async (events, clientId) => {
        const pub1 = await publishRandom(events, clientId)
        const request: SubscribeMonotonicRequest = {
          session: 'test-session',
          query: `FROM '${mySuite()}' & '${testName()}' & 'client:${clientId}' & isLocal`,
          lowerBound: {},
        }
        const data: SubscribeMonotonicResponse[] = []
        const done = new Promise((resolve, reject) => {
          events
            .subscribeMonotonic(request, (x) => {
              data.push(x)
              if (data.length == 3) {
                resolve()
              }
            })
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })
        await waitFor(
          () => {
            expect(data.length).toBeGreaterThanOrEqual(2)
          },
          5_000,
          50,
        )
        const pub2 = await publishRandom(events, clientId)
        await done
        expect(data).toMatchObject([
          pub1,
          { type: 'offsets', offsets: { [pub1.stream]: expect.any(Number) } },
          pub2,
        ])
        expect(data[1].type === 'offsets' && data[1].offsets[pub1.stream]).toBeGreaterThanOrEqual(
          pub1.offset,
        )
        expect(data[1].type === 'offsets' && data[1].offsets[pub1.stream]).toBeLessThan(pub2.offset)
      }))
  })
})
