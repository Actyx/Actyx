import { quickstartDirs } from '../../setup-projects/quickstart'
import { settings } from '../../infrastructure/settings'
import { runOnEvery } from '../../infrastructure/hosts'
import { waitFor, waitForAppToStop } from '../../retry'
import path from 'path'
import { Arch } from '../../../jest/types'
import { assertOK } from '../../assertOK'
import { stubs } from '../../stubs'
import { tempDir } from '../../setup-projects/util'

const projectTempDir = path.resolve(settings().tempDir)

describe('basic app lifecycle', () => {
  test('for quickstart sample-docker-app run deploy, start, ls, stop, undeploy', async () => {
    const workingDir = tempDir()
    const projectDir = quickstartDirs(projectTempDir).sampleDockerApp
    console.log('package cwd', workingDir, path.resolve(projectDir, 'ax-manifest-all.yml'))
    const pkgResponse = assertOK(
      await stubs.axOnly.ax.apps.packageCwd(
        workingDir,
        path.resolve(projectDir, 'ax-manifest-all.yml'),
      ),
    )
    expect(pkgResponse.result).toHaveLength(2)

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
      if (node.target.arch === 'armv7') {
        // ax cannot yet package for 32bit arm
        return
      }

      const responseDeploy = await node.ax.apps.deploy(packagePath(node.target.arch))
      expect(responseDeploy).toMatchCodeOk()

      const responseStart = await node.ax.apps.start(appId)
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
        const response = await node.ax.apps.ls()
        expect(response).toMatchObject(appRunning)
      })

      const responseStop = await node.ax.apps.stop(appId)
      expect(responseStop).toMatchCodeOk()
      await waitForAppToStop(appId, node)

      const responseUndeploy = await node.ax.apps.undeploy(appId)
      expect(responseUndeploy).toMatchCodeOk()

      expect(await node.ax.apps.ls()).toMatchObject({ code: 'OK', result: [] })
    })
  }, 240_000)
})
