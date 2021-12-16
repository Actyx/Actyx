import { SubscribeResponse } from '../../http-client'
import { waitFor } from '../../retry'
import { runWithClients } from '../../util'
import { genericCommunicationTimeout, mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('subscribe', () => {
    it('should publish an event and find it in an event stream', () =>
      runWithClients(async (events, clientId) => {
        const pub1 = await publishRandom(events, clientId)
        const data: SubscribeResponse[] = []
        await new Promise<void>((resolve, reject) => {
          events
            .subscribe(
              { query: `FROM '${mySuite()}' & '${testName()}' & 'client:${clientId}' & isLocal` },
              (res, cancel) => {
                data.push(res)
                if (data.length == 2) {
                  cancel()
                  resolve()
                }
              },
            )
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })
        expect(data).toMatchObject([
          pub1,
          { type: 'offsets', offsets: { [pub1.stream]: expect.any(Number) } },
        ])
      }))

    it('should start an event stream and find a published event', () =>
      runWithClients(async (events, clientId) => {
        const pub1 = await publishRandom(events, clientId)
        const data: SubscribeResponse[] = []
        const done = new Promise<void>((resolve, reject) => {
          events
            .subscribe(
              { query: `FROM '${mySuite()}' & '${testName()}' & 'client:${clientId}' & isLocal` },
              (res, cancel) => {
                data.push(res)
                if (data.length == 3) {
                  cancel()
                  resolve()
                }
              },
            )
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

    // TODO: test subscription across Actyx node restart
  })
})
