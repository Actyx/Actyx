/**
 * @jest-environment ./dist/jest/environment
 */
import { runOnEach } from '../../infrastructure/hosts'

describe('Logging on Android', () => {
  test('should write stuff to logcat', async () => {
    await runOnEach([{ host: 'android' }], async () => {
      // qed by having a running android node
    })
  })
})
