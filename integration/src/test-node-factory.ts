/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { createNode } from './infrastructure/create'
import { ActyxOSNode } from './infrastructure/types'

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
