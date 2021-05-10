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
        await new Promise((resolve) => {
          es.subscribe({ query: `FROM '${integrationTag}' & 'test:1'` }, (x) => data.push(x))
          setTimeout(resolve, genericCommunicationTimeout)
        })

        const ev = data.find((x) => x.lamport === pub1.lamport)
        expect(ev).toMatchObject(pub1)
      }))
  })
})
