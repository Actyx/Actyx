import { mkESFromTrial, SubscribeResponse } from '../../http-client'
import { run } from '../../util'
import { genericCommunicationTimeout, integrationTag, publishRandom } from './utils.support.test'

describe('event service', () => {
  describe('subscribe to event streams', () => {
    it('should publish event and find it an event stream', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)

        const data: SubscribeResponse[] = []
        await new Promise((resolve, reject) => {
          es.subscribe({ query: `FROM '${integrationTag}' & 'test:1' & isLocal` }, (x) =>
            data.push(x),
          )
            .then(resolve)
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })

        const ev = data.find((x) => x.lamport === pub1.lamport)
        expect(ev).toMatchObject(pub1)
      }))

    it('should start an event stream and find a published event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)

        const data: SubscribeResponse[] = []
        const done = new Promise((resolve, reject) => {
          es.subscribe({ query: `FROM '${integrationTag}' & 'test:1' & isLocal` }, (x) =>
            data.push(x),
          )
            .then(resolve)
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })

        const pub1 = await publishRandom(es)
        await done

        const ev = data.find((x) => x.lamport === pub1.lamport)
        expect(ev).toMatchObject(pub1)
      }))
  })
})
