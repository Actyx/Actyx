import { remove } from 'fs-extra'
import { stubNode } from '../../stubs'
import { SettingsInput } from '../exec'
import { quickstartDirs } from '../setup-projects/quickstart'

describe('start', () => {
  const scope = 'com.actyx.os'
  const appId = 'com.actyx.sample-docker-app'
  const tarballFile = 'com.actyx.sample-docker-app-1.0.0.tar.gz'

  const removeTarballs = async () => {
    await remove(`${tarballFile}`)
    await remove(`${quickstartDirs.sampleDockerApp}/${tarballFile}`)
  }

  const reset = async () => {
    await removeTarballs()
    await stubNode.ax.Settings.Unset(scope)
  }

  beforeEach(async () => reset())

  afterEach(async () => reset())

  test('quickstart/sample-docker-app run deploy/start/ls/stop/undeploy', async () => {
    await stubNode.ax.Settings.Set(
      scope,
      SettingsInput.FromFile(`${quickstartDirs.quickstart}/misc/local-sample-node-settings.yml`),
    )
    const responseDeploy = await stubNode.ax.Apps.Deploy(quickstartDirs.sampleDockerApp)
    expect(responseDeploy).toMatchCodeOk()

    const responseStart = await stubNode.ax.Apps.Start(appId)
    expect(responseStart).toMatchCodeOk()

    const responseLsAfterStart = await stubNode.ax.Apps.Ls()
    expect(responseLsAfterStart).toMatchCodeOk()

    const responseStop = await stubNode.ax.Apps.Stop(appId)
    expect(responseStop).toMatchCodeOk()

    await new Promise((res) =>
      setTimeout(async () => {
        await stubNode.ax.Apps.Undeploy(appId)
        const responseLs2AfterUndeploy = await stubNode.ax.Apps.Ls()
        const responseLs2AfterUndeployShape = { code: 'OK', result: [] }
        expect(responseLs2AfterUndeploy).toMatchObject(responseLs2AfterUndeployShape)
        res()
      }, 4000),
    )
  })
})
