import { remove } from 'fs-extra'
import { stubNode } from '../../../stubs'
import { quickstartDirs } from '../../../setup-projects/quickstart'
import { resetTestEviroment, waitForStop } from '../../local-docker-util'

const WAIT_TIMEOUT_MS = 20_000
const TRY_FREQUENCY_MS = 1_000

const APP_ID = 'com.actyx.sample-docker-app'
const TARBALL_FILE = 'com.actyx.sample-docker-app-1.0.0-x86_64.tar.gz' // SPO double check this

const waitStop = waitForStop(TRY_FREQUENCY_MS, WAIT_TIMEOUT_MS)
const waitStopDockerApp = waitStop('com.actyx.sample-docker-app')

describe('quickstart-dockerapp', () => {
  const reset = async () => {
    await remove(`${TARBALL_FILE}`)
    await resetTestEviroment()
  }

  beforeEach(async () => await reset())

  afterEach(async () => await reset())

  test('quickstart/sample-docker-app run deploy/start/ls/stop/undeploy', async () => {
    const responseDeploy = await stubNode.ax.Apps.Deploy(quickstartDirs.sampleDockerApp)
    expect(responseDeploy).toMatchCodeOk()

    const responseStart = await stubNode.ax.Apps.Start(APP_ID)
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

    const responseStop = await stubNode.ax.Apps.Stop(APP_ID)
    expect(responseStop).toMatchCodeOk()

    await waitStopDockerApp()

    const responseUndeploy = await stubNode.ax.Apps.Undeploy(APP_ID)
    expect(responseUndeploy).toMatchCodeOk()

    const responseLs2 = await stubNode.ax.Apps.Ls()
    const responseLs2Shape = { code: 'OK', result: [] }
    expect(responseLs2).toMatchObject(responseLs2Shape)
  })
})
