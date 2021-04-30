import { publishEvent, randomString } from '../../util'

describe.skip('event service', () => {
  describe('publish events', () => {
    it('should publish event', async () => {
      const publishRes = await publishEvent(randomString())

      expect(publishRes.status).toBe(200)
      expect(publishRes.data).toMatchObject({
        data: [
          {
            lamport: expect.any(Number),
            stream: expect.any(String),
            offset: expect.any(Number),
          },
        ],
      })
    })

    it('should increase offset for new published event', async () => {
      const publishRes1 = await publishEvent(randomString())
      expect(publishRes1.status).toBe(200)

      const publishRes2 = await publishEvent(randomString())
      expect(publishRes2.status).toBe(200)

      const offsetEvent1 = publishRes1.data.data[0].offset
      const offsetEvent2 = publishRes2.data.data[0].offset
      expect(offsetEvent2 > offsetEvent1).toBeTruthy()
    })
  })
})
