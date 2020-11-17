import { stubNode } from '../../stubs'
import { quickstartDirs } from '../../setup-projects/quickstart'
import { settings } from '../../infrastructure/settings'
import { runOnEvery } from '../../infrastructure/hosts'
import { waitFor } from '../../retry'
import path from 'path'

const tempDir = settings().tempDir

describe('basic app lifecycle', () => {
  test('for quickstart sample-docker-app run deploy, start, ls, stop, undeploy', async () => {
    const cwd = quickstartDirs(tempDir).sampleDockerApp
    const pkgResponse = await stubNode.ax.Apps.PackageCwd(cwd)
    if (pkgResponse.code !== 'OK') {
      fail(`failed to package quickstart docker: ${JSON.stringify(pkgResponse)}`)
    }
    const { appId, appVersion, packagePath: packageName } = pkgResponse.result[0]
    const packagePath = path.resolve(cwd, packageName)

    expect(appVersion).toMatch(/^1\.\d+\.\d+$/)

    await runOnEvery({ runtime: 'docker' }, async (node) => {
      const responseDeploy = await node.ax.Apps.Deploy(packagePath)
      expect(responseDeploy).toMatchCodeOk()

      const responseStart = await node.ax.Apps.Start(appId)
      expect(responseStart).toMatchCodeOk()

      const appRunning = {
        code: 'OK',
        result: [
          {
            appId,
            version: '1.0.0',
            running: true,
            licensed: true,
            settingsValid: true,
            enabled: true,
          },
        ],
      }
      await waitFor(async () => {
        const response = await node.ax.Apps.Ls()
        expect(response).toMatchObject(appRunning)
      })

      const responseStop = await node.ax.Apps.Stop(appId)
      expect(responseStop).toMatchCodeOk()

      await waitFor(async () => {
        const response = await node.ax.Apps.Ls()
        expect(response).toMatchObject({
          code: 'OK',
          result: [{ enabled: false, running: false }],
        })
      }, 15_000)

      const responseUndeploy = await node.ax.Apps.Undeploy(appId)
      expect(responseUndeploy).toMatchCodeOk()

      expect(await node.ax.Apps.Ls()).toMatchObject({ code: 'OK', result: [] })
    })
  })
})
