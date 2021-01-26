import { ActyxOSNode } from './infrastructure/types'

const million = BigInt(1_000_000)
const millisToBigInt = (n: number) => BigInt(n) * million

/**
 * Waits for the expectation to pass and returns a Promise
 *
 * @param  expectation  Function  Expectation that has to complete without throwing
 * @param  timeout  Number  Maximum wait interval, 10s by default
 * @param  wait_period  Number  Wait-between-retries interval, 500ms by default
 * @return  Promise  Promise to return a callback result
 */
export const waitFor = <T>(
  expectation: () => T | Promise<T>,
  timeout = 10_000,
  wait_period = 500,
): Promise<T> => {
  const deadline = process.hrtime.bigint() + millisToBigInt(timeout)
  return new Promise<T>((resolve, reject) => {
    const runExpectation = async () => {
      try {
        resolve(await expectation())
      } catch (error) {
        if (process.hrtime.bigint() > deadline) {
          reject(error)
          return
        }
        setTimeout(runExpectation, wait_period)
      }
    }
    setTimeout(runExpectation, 0)
  })
}

export const waitForNodeToBeConfigured = async (node: ActyxOSNode): Promise<void> =>
  await waitFor(async () => {
    const response = await node.ax.nodes.ls()
    if (response.code == 'OK') {
      expect(response).toMatchObject({
        code: 'OK',
        result: [{ ...response.result[0], connection: 'reachable', settingsValid: true }],
      })
    } else {
      expect(false)
    }
  })

export const retryTimes = async <T>(op: () => T | Promise<T>, times: number): Promise<T> => {
  let tries = 1
  for (;;) {
    try {
      return await op()
    } catch (error) {
      if (tries >= times) {
        throw error
      } else {
        tries += 1
      }
    }
  }
}
