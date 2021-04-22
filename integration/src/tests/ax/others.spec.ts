import { runOnEach } from '../../infrastructure/hosts'

describe('ax', () => {
  describe('version', () => {
    it('should return the right version number', async () => {
      await runOnEach([{}], async (node) => {
        const response = await node.ax.version()
        expect(response.startsWith('Actyx CLI 2.0.0')).toBeTruthy()
      })
    })
  })
})
