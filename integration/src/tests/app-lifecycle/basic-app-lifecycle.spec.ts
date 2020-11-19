import { stubNode } from '../../stubs'
import { quickstartDirs } from '../../setup-projects/quickstart'
import { settings } from '../../infrastructure/settings'
import { runOnEvery } from '../../infrastructure/hosts'
import { waitFor } from '../../retry'
import path from 'path'
import { Arch } from '../../../jest/types'

const tempDir = settings().tempDir

describe('basic app lifecycle', () => {
  test('for quickstart sample-docker-app run deploy, start, ls, stop, undeploy', async () => {
    const workingDir = quickstartDirs(tempDir).sampleDockerApp
    const pkgResponse = await stubNode.ax.Apps.PackageCwd(workingDir)
    if (pkgResponse.code !== 'OK') {
      fail(`failed to package quickstart docker: ${JSON.stringify(pkgResponse)}`)
    }
    const { appId, appVersion } = pkgResponse.result[0]
    for (const pkg of pkgResponse.result.slice(1)) {
      expect(pkg).toMatchObject({ appId, appVersion })
    }
    const packagePath = (arch: Arch) => {
      const found = pkgResponse.result.find(({ packagePath }) => packagePath.indexOf(arch) >= 0)
      if (found === undefined) {
        fail(
          `no package for ${arch} in [${pkgResponse.result.map((x) => x.packagePath).join(', ')}]`,
        )
      }
      return path.resolve(workingDir, found.packagePath)
    }

    expect(appVersion).toMatch(/^1\.\d+\.\d+$/)

    await runOnEvery({ runtime: 'docker' }, async (node) => {
      const responseDeploy = await node.ax.Apps.Deploy(packagePath(node.target.arch))
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
  }, 240_000)
})
