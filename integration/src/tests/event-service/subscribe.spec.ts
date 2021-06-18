import { mkESFromTrial, SubscribeResponse } from '../../http-client'
import { run } from '../../util'
import { genericCommunicationTimeout, mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('subscribe to event streams', () => {
    it('should publish an event and find it in an event stream', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const pub1 = await publishRandom(es)

        const ev = await new Promise<SubscribeResponse>((resolve, reject) => {
          es.subscribe({ query: `FROM '${mySuite()}' & '${testName()}' & isLocal` }, resolve).catch(
            reject,
          )
          setTimeout(reject, genericCommunicationTimeout)
        })

        expect(ev.appId).toEqual('com.example.my-app')
        expect(ev).toMatchObject(pub1)
      }))

    it('should start an event stream and find a published event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)

        const done = new Promise<SubscribeResponse>((resolve, reject) => {
          es.subscribe({ query: `FROM '${mySuite()}' & '${testName()}' & isLocal` }, resolve).catch(
            reject,
          )
          setTimeout(reject, genericCommunicationTimeout)
        })

        const pub1 = await publishRandom(es)
        const ev = await done

        expect(ev.appId).toEqual('com.example.my-app')
        expect(ev).toMatchObject(pub1)
      }))
  })
})
