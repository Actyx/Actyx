import {
  mkESFromTrial,
  SubscribeMonotonicRequest,
  SubscribeMonotonicResponse,
} from '../../http-client'
import { run } from '../../util'
import { genericCommunicationTimeout, integrationTag, publishRandom } from './utils.support.test'

// TODO: make this work or find the bug
describe.skip('event service', () => {
  describe('subscribe_monotonic', () => {
    it('should publish event and find it in a monotonic event stream', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)

        const request: SubscribeMonotonicRequest = {
          session: 'test-session',
          where: `'${integrationTag}'`,
          offsets: { [pub1.stream]: 0 },
        }

        const data: SubscribeMonotonicResponse[] = []
        await new Promise((resolve) => {
          es.subscribeMonotonic(request, (x) => data.push(x))
          setTimeout(resolve, genericCommunicationTimeout)
        })

        console.log(data)
        // const ev = data.find((x) => x.lamport === pub1.lamport)
        // expect(ev).toMatchObject(pub1)
      }))
  })
})
