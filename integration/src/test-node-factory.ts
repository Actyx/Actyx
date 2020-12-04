/* eslint-disable @typescript-eslint/no-non-null-assertion */
import path from 'path'
import { assertOK } from './assertOK'
import { createNode } from './infrastructure/create'
import { settings } from './infrastructure/settings'
import { ActyxOSNode } from './infrastructure/types'
import { quickstartDirs } from './setup-projects/quickstart'

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
    const tempDir = settings().tempDir
    const workingDir = quickstartDirs(tempDir).sampleDockerApp
    const responsePacakge = assertOK(await node.ax.apps.packageCwd(workingDir, 'ax-manifest.yml'))
    const { packagePath: responsePacakgePath, appId: responseAppId } = responsePacakge.result[0]
    const packagePath = path.resolve(workingDir, responsePacakgePath)
    const appId = responseAppId
    return Promise.resolve({
      appId,
      packagePath,
    })
  } catch (err) {
    throw new Error(err)
  }
}
