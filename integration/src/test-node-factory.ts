import { MyGlobal } from '../jest/setup'
import { createNode, mkActyxNodeWithLogging, mkLocalTarget } from './infrastructure/create'
import { mkNodeLocalProcess } from './infrastructure/local'
import { ActyxNode } from './infrastructure/types'

const createTestNode = async (name: string, install: 'docker' | 'linux'): Promise<ActyxNode> => {
  const testNode = await createNode({
    name,
    install: { type: install },
    prepare: { type: 'local' },
  })
  return testNode
}

export const createTestNodeDockerLocal = (nodeName: string): Promise<ActyxNode> => {
  const prefix = 'test-node-local-docker'
  const name = `${prefix}-${nodeName}`
  return createTestNode(name, 'docker')
}

export const createTestNodeLocal = (nodeName: string): Promise<ActyxNode> => {
  const prefix = 'test-node-local-linux'
  const name = `${prefix}-${nodeName}`
  return createTestNode(name, 'linux')
}

export const createLocalTestNode = async (
  nodeName: string,
  reuseWorkingDirIfExists?: boolean,
): Promise<ActyxNode> => {
  const target = mkLocalTarget(nodeName, reuseWorkingDirIfExists)
  const {
    settings: { logToStdout },
    runIdentifier,
  } = (<MyGlobal>global).axNodeSetup
  const node = await mkActyxNodeWithLogging(
    runIdentifier,
    logToStdout,
    nodeName,
    mkNodeLocalProcess(nodeName, target, reuseWorkingDirIfExists),
  )

  return node
}
