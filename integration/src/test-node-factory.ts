/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { createNode } from './infrastructure/create'
import { ActyxOSNode } from './infrastructure/types'

export const createTestNode = async (
  name: string,
  install: 'docker' | 'linux',
): Promise<ActyxOSNode> => {
  const testNode = await createNode({
    name,
    install,
    prepare: {
      type: 'local',
    },
  })
  if (testNode === undefined) {
    throw new Error(`could not create ${name}`)
  }
  return Promise.resolve(testNode)
}

export const createTestNodeDockerLocal = async (nodeName: string): Promise<ActyxOSNode> => {
  const prefix = 'test-node-local-docker'
  const name = `${prefix}-${nodeName}`
  return createTestNode(name, 'docker')
}

export const createTestNodeLocal = async (nodeName: string): Promise<ActyxOSNode> => {
  const prefix = 'test-node-local-linux'
  const name = `${prefix}-${nodeName}`
  return createTestNode(name, 'linux')
}
