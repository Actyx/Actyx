/**
 * @jest-environment ./dist/jest/environment
 */
import { runWithClients } from '../../util'
import { mySuite, publishRandom, testName } from './utils.support.test'

describe('event service', () => {
  describe('publish', () => {
    it('should publish event', () =>
      runWithClients(async (events, clientId) => {
        const result = await events.publish({
          data: [
            { tags: [mySuite(), testName(), `client:${clientId}`, 'tag'], payload: { foo: 'bar' } },
          ],
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
      runWithClients(async (events, clientId) => {
        const ev1 = await publishRandom(events, clientId)
        const ev2 = await publishRandom(events, clientId)
        expect(ev1.lamport).toBeLessThan(ev2.lamport)
        expect(ev1.offset).toBeLessThan(ev2.offset)
        expect(ev1.timestamp).toBeLessThan(ev2.timestamp)
      }))
  })
})
