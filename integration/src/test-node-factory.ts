import { createNode } from './infrastructure/create'
import { ActyxNode } from './infrastructure/types'

const createTestNode = async (name: string, install: 'docker' | 'linux'): Promise<ActyxNode> => {
  const testNode = await createNode({ name, install, prepare: { type: 'local' } })
  if (testNode === undefined) {
    throw new Error(`could not create ${name}`)
  }
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
