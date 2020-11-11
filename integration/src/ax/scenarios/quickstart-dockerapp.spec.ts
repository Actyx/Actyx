import { remove } from 'fs-extra'
import { waitForMs } from '../../jest-util'
import { stubNode } from '../../stubs'
import { SettingsInput } from '../exec'
import { quickstartDirs } from '../setup-projects/quickstart'

describe('start', () => {
  const scope = 'com.actyx.os'
  const appId = 'com.actyx.sample-docker-app'
  const tarballFile = 'com.actyx.sample-docker-app-1.0.0.tar.gz'

  const undeployAllApps = async () => {
    // TODO: SPO make sure all apps are undeployed and minimize jest timeout
    const responseLs = await stubNode.ax.Apps.Ls()
    if (responseLs.code === 'OK') {
      const appIds = responseLs.result.map((r) => r.appId)
      console.log(appIds)
      appIds.forEach((id) => stubNode.ax.Apps.Undeploy(id))
      await waitForMs(6000)
    }
    expect(responseLs).toMatchCodeOk()
  }

  const reset = async () => {
    await remove(`${tarballFile}`)
    await stubNode.ax.Settings.Unset(scope)
    await undeployAllApps()
  }

  beforeEach(async () => await reset())

  afterEach(async () => await reset())

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
    expect(responseLs1).toMatchCodeOk()

    const responseStop = await stubNode.ax.Apps.Stop(appId)
    expect(responseStop).toMatchCodeOk()

    await waitForMs(6000)

    await stubNode.ax.Apps.Undeploy(appId)
    const responseLs2 = await stubNode.ax.Apps.Ls()
    const responseLs2Shape = { code: 'OK', result: [] }
    expect(responseLs2).toMatchObject(responseLs2Shape)
  })
})
