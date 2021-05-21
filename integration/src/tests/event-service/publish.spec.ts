import { mkESFromTrial } from '../../http-client'
import { run } from '../../util'
import { mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('publish', () => {
    it('should publish event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const result = await es.publish({
          data: [{ tags: [mySuite(), testName(), 'tag'], payload: { foo: 'bar' } }],
        })

        expect(result).toMatchObject({
          data: [
            {
              lamport: expect.any(Number),
              stream: expect.any(String),
              offset: expect.any(Number),
              timestamp: expect.any(Number),
            },
          ],
        })
      }))

    it('should increase offset for new published event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const ev1 = await publishRandom(es)
        const ev2 = await publishRandom(es)
        expect(ev1.lamport).toBeLessThan(ev2.lamport)
        expect(ev1.offset).toBeLessThan(ev2.offset)
        expect(ev1.timestamp).toBeLessThan(ev2.timestamp)
      }))
  })
})
