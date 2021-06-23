import { mkESFromTrial, SubscribeResponse } from '../../http-client'
import { run } from '../../util'
import { genericCommunicationTimeout, mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('subscribe', () => {
    it('should publish an event and find it in an event stream', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const data: SubscribeResponse[] = []
        await new Promise((resolve, reject) => {
          es.subscribe({ query: `FROM '${mySuite()}' & '${testName()}' & isLocal` }, (x) => {
            data.push(x)
            if (data.length == 2) {
              resolve()
            }
          }).catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })
        expect(data).toMatchObject([
          pub1,
          { type: 'offsets', offsets: { [pub1.stream]: expect.any(Number) } },
        ])
      }))

    it('should start an event stream and find a published event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)
        const data: SubscribeResponse[] = []
        const done = new Promise((resolve, reject) => {
          es.subscribe({ query: `FROM '${mySuite()}' & '${testName()}' & isLocal` }, (x) => {
            data.push(x)
            if (data.length == 3) {
              resolve()
            }
          }).catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })
        const pub2 = await publishRandom(es)
        await done
        expect(data).toMatchObject([
          pub1,
          { type: 'offsets', offsets: { [pub1.stream]: expect.any(Number) } },
          pub2,
        ])
      }))
  })
})
