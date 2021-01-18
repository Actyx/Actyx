/* eslint-disable @typescript-eslint/no-non-null-assertion */
import path from 'path'
import { assertOK } from './assertOK'
import { createNode } from './infrastructure/create'
import { settings } from './infrastructure/settings'
import { ActyxOSNode } from './infrastructure/types'
import { quickstartDirs } from './setup-projects/quickstart'
import { tempDir } from './setup-projects/util'

export const createTestNodeDockerLocal = async (nodeName: string): Promise<ActyxOSNode> => {
  const prefix = 'test-node-local-docker'
  const name = `${prefix}-${nodeName}`
  const testNode = await createNode({
    name,
    install: 'docker',
    prepare: {
      type: 'local',
    },
  })
  if (testNode === undefined) {
    throw new Error(`could not create ${name}`)
  }
  return Promise.resolve(testNode)
}

export const createPackageSampleDockerApp = async (
  node: ActyxOSNode,
): Promise<{ appId: string; packagePath: string }> => {
  try {
    const workingDir = tempDir()
    const projectTempDir = path.resolve(settings().tempDir)
    const appDir = quickstartDirs(projectTempDir).sampleDockerApp
    const responsePackage = assertOK(
      await node.ax.apps.packageCwd(workingDir, path.resolve(appDir, 'ax-manifest.yml')),
    )
    const { packagePath: responsePackagePath, appId: responseAppId } = responsePackage.result[0]
    const packagePath = path.resolve(workingDir, responsePackagePath)
    const appId = responseAppId
    return Promise.resolve({
      appId,
      packagePath,
    })
  } catch (err) {
    throw new Error(err)
  }
}
