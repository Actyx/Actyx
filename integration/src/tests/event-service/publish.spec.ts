import { mkESFromTrial } from '../../http-client'
import { run } from '../../util'
import { publishRandom } from './utils.support.test'

describe('event service', () => {
  describe('publish', () => {
    it('should publish event', () =>
      run(async (x) => {
        const es = await mkESFromTrial(x)
        const result = await es.publish({ data: [{ tags: ['tag'], payload: { foo: 'bar' } }] })

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
        const result1 = await publishRandom(es)
        const result2 = await publishRandom(es)
        const offsetEvent1 = result1.offset
        const offsetEvent2 = result2.offset
        expect(offsetEvent2 > offsetEvent1).toBeTruthy()
      }))
  })
})
