import { remove } from 'fs-extra'
import { stubNode } from '../stubs'
import { waitForStop } from '../jest-util'
import { SettingsInput } from '../cli/exec'
import { quickstartDirs } from '../setup-projects/quickstart'
import { Response_Settings_Set, Response_Settings_Unset } from '../cli/types'

const WAIT_TIMEOUT_MS = 20_000
const TRY_FREQUENCY_MS = 1_000

const waitStop = waitForStop(TRY_FREQUENCY_MS, WAIT_TIMEOUT_MS)

const ACTYXOS_SCOPE = 'com.actyx.os'
const tarballFile = 'com.actyx.sample-docker-app-1.0.0-x86_64.tar.gz' // SPO double check this

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
          await waitStop(a.appId)()
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

  // await remove(`${tarballFile}`)

  await unsetNode(ACTYXOS_SCOPE)
  await setNode(ACTYXOS_SCOPE)
}
