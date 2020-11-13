import { stubNode } from '../stubs'
import { SettingsInput } from '../cli/exec'
import { quickstartDirs } from '../setup-projects/quickstart'
import { Response_Settings_Set, Response_Settings_Unset } from '../cli/types'

const ACTYXOS_SCOPE = 'com.actyx.os'
const WAIT_TIMEOUT_MS = 20_000
const TRY_FREQUENCY_MS = 1_000

export const waitForActyxOStoBeReachable = async (): Promise<void> => {
  const predicate = async (): Promise<boolean> => {
    const resultNodeLs = await stubNode.ax.Nodes.Ls()
    if (resultNodeLs.code === 'OK') {
      const isRechable = resultNodeLs.result[0].connection === 'reachable'
      return isRechable
    } else {
      return false
    }
  }
  await waitForByInterval(TRY_FREQUENCY_MS, WAIT_TIMEOUT_MS)(predicate)()
}

export const waitForByInterval = (checkEveryMs: number, timeoutMs: number) => (
  predicateFb: () => Promise<boolean>,
) => (): Promise<void> => {
  const started = process.hrtime()
  return new Promise((res, rej) => {
    const check = () => {
      const [diffSeconds] = process.hrtime(started)
      if (diffSeconds >= timeoutMs / 1000) {
        rej('waitForStop timeout')
        return
      }
      setTimeout(async () => {
        const canResolve = await predicateFb()
        console.log('canResolve', canResolve)
        if (canResolve) {
          res()
          return
        } else {
          check()
        }
      }, checkEveryMs)
    }

    check()
  })
}

export const waitForStop = async (appId: string): Promise<void> => {
  const predicate = (appId: string) => async (): Promise<boolean> => {
    const resultLs = await stubNode.ax.Apps.Ls()
    console.log('resultSTOP', JSON.stringify(resultLs))
    if (resultLs.code === 'OK') {
      const app = resultLs.result.find((a) => a.appId === appId)
      const isAppStopped = app?.running === false
      return isAppStopped
    } else {
      return false
    }
  }
  return waitForByInterval(TRY_FREQUENCY_MS, WAIT_TIMEOUT_MS)(predicate(appId))()
}

const stopAndUndeployAllApps = async () => {
  const responseLs = await stubNode.ax.Apps.Ls()
  if (responseLs.code === 'OK') {
    const apps = responseLs.result.map((r) => ({
      appId: r.appId,
      running: r.running,
    }))
    const hasApps = apps.length > 0
    if (hasApps) {
      const appsRunning = apps.filter((a) => a.running === true)
      const hasAppsRunning = appsRunning.length > 0
      if (hasAppsRunning) {
        appsRunning.forEach(async (a) => {
          await stubNode.ax.Apps.Stop(a.appId)
          await waitForStop(a.appId)
        })
      }
      apps.forEach((app) => stubNode.ax.Apps.Undeploy(app.appId))
    }
  }
}

const setNode = async (scope: string): Promise<Response_Settings_Set> =>
  await stubNode.ax.Settings.Set(
    scope,
    SettingsInput.FromFile(`${quickstartDirs.quickstart}/misc/local-sample-node-settings.yml`),
  )

const unsetNode = async (scope: string): Promise<Response_Settings_Unset> =>
  await stubNode.ax.Settings.Unset(scope)

export const resetTestEviroment = async (): Promise<void> => {
  await stopAndUndeployAllApps()

  await unsetNode(ACTYXOS_SCOPE)
  await setNode(ACTYXOS_SCOPE)
}
