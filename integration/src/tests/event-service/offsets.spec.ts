/**
 * @jest-environment ./dist/integration/src/jest/environment
 */
import { runWithClients } from '../../util'

describe('event service', () => {
  describe('offsets', () => {
    it('get', () =>
      runWithClients(async (events) => {
        const offsets = await events.offsets()
        expect(offsets).toMatchObject({
          present: expect.any(Object),
          toReplicate: expect.any(Object),
        })
        expect(Object.keys(offsets.present).length).toBeGreaterThan(0)
      }))
  })
})
