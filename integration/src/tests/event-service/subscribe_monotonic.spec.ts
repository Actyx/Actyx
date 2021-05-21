import {
  mkESFromTrial,
  SubscribeMonotonicRequest,
  SubscribeMonotonicResponse,
} from '../../http-client'
import { run } from '../../util'
import { genericCommunicationTimeout, mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('subscribe_monotonic', () => {
    it('should publish event and find it in a monotonic event stream', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)

        const request: SubscribeMonotonicRequest = {
          session: 'test-session',
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          offsets: {},
        }

        const data: SubscribeMonotonicResponse[] = []
        await new Promise((resolve, reject) => {
          es.subscribeMonotonic(request, (x) => data.push(x))
            .then(resolve)
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })

        const ev = data.find((x) => x.type === 'event' && x.lamport === pub1.lamport)
        if (ev === undefined) {
          console.log(data)
        }
        expect(ev).toMatchObject(pub1)
      }))

    it('should start a monotonic event stream and find published event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)

        const request: SubscribeMonotonicRequest = {
          session: 'test-session',
          query: `FROM '${mySuite()}' & '${testName()}' & isLocal`,
          offsets: {},
        }

        const data: SubscribeMonotonicResponse[] = []
        const done = new Promise((resolve, reject) => {
          es.subscribeMonotonic(request, (x) => data.push(x))
            .then(resolve)
            .catch(reject)
          setTimeout(resolve, genericCommunicationTimeout)
        })

        const pub1 = await publishRandom(es)
        await done

        const ev = data.find((x) => x.type === 'event' && x.lamport === pub1.lamport)
        if (ev === undefined) {
          console.log(data)
        }
        expect(ev).toMatchObject(pub1)
      }))
  })
})
