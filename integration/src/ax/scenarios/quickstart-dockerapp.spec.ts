import { remove } from 'fs-extra'
import { waitForStop } from '../../jest-util'
import { stubNode } from '../../stubs'
import { SettingsInput } from '../exec'
import { quickstartDirs } from '../setup-projects/quickstart'

const WAIT_TIMEOUT_MS = 20_000
const TRY_FREQUENCY_MS = 1_000

const waitStop = waitForStop(TRY_FREQUENCY_MS, WAIT_TIMEOUT_MS)
const waitStopDockerApp = waitStop('com.actyx.sample-docker-app')

describe('quickstart-dockerapp', () => {
  const scope = 'com.actyx.os'
  const appId = 'com.actyx.sample-docker-app'
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
            stubNode.ax.Apps.Stop(a.appId)
            await waitStop(a.appId)()
          })
        }
        apps.forEach((app) => stubNode.ax.Apps.Undeploy(app.appId))
      }
    }
    expect(responseLs).toMatchCodeOk()
  }

  const resetTestEviroment = async () => {
    await stubNode.ax.Settings.Unset(scope)
    await remove(`${tarballFile}`)
    await stopAndUndeployAllApps()
  }

  beforeEach(async () => await resetTestEviroment())

  afterEach(async () => await resetTestEviroment())

  test('quickstart/sample-docker-app run deploy/start/ls/stop/undeploy', async () => {
    await stubNode.ax.Settings.Set(
      scope,
      SettingsInput.FromFile(`${quickstartDirs.quickstart}/misc/local-sample-node-settings.yml`),
    )
    const responseDeploy = await stubNode.ax.Apps.Deploy(quickstartDirs.sampleDockerApp)
    expect(responseDeploy).toMatchCodeOk()

    const responseStart = await stubNode.ax.Apps.Start(appId)
    expect(responseStart).toMatchCodeOk()

    const responseLs1 = await stubNode.ax.Apps.Ls()
    const responseLs1Shape = {
      code: 'OK',
      result: [
        {
          nodeId: 'localhost',
          appId: 'com.actyx.sample-docker-app',
          version: '1.0.0',
          running: false,
          startedIso: null,
          startedUnix: null,
          licensed: true,
          settingsValid: true,
          enabled: true,
        },
      ],
    }
    expect(responseLs1).toMatchObject(responseLs1Shape)

    const responseStop = await stubNode.ax.Apps.Stop(appId)
    expect(responseStop).toMatchCodeOk()

    await waitStopDockerApp()

    const responseUndeploy = await stubNode.ax.Apps.Undeploy(appId)
    expect(responseUndeploy).toMatchCodeOk()

    const responseLs2 = await stubNode.ax.Apps.Ls()
    const responseLs2Shape = { code: 'OK', result: [] }
    expect(responseLs2).toMatchCodeOk()
    expect(responseLs2).toMatchObject(responseLs2Shape)
  })
})
