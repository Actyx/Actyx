import { stubNode } from './stubs'

export const waitForStop = (checkEveryMs: number, timeoutMs: number) => (
  appId: string,
) => (): Promise<string> => {
  const started = process.hrtime()
  return new Promise((res, rej) => {
    const check = () => {
      const [diffSeconds] = process.hrtime(started)
      if (diffSeconds >= timeoutMs / 1000) {
        rej('waitForStop timeout')
        return
      }
      setTimeout(async () => {
        const resultLs = await stubNode.ax.Apps.Ls()
        if (resultLs.code === 'OK') {
          const app = resultLs.result.find((a) => a.appId === appId)
          const isAppStopped = app?.running === false
          if (isAppStopped) {
            res(`${app?.appId} is stopped`)
            return
          } else {
            check()
          }
        }
      }, checkEveryMs)
    }

    check()
  })
}
