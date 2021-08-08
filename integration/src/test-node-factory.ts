import { createNode } from './infrastructure/create'
import { ActyxNode } from './infrastructure/types'

export const createTestNodeDockerLocal = (nodeName: string): Promise<ActyxNode> => {
  const prefix = 'test-node-local-docker'
  const name = `${prefix}-${nodeName}`
  return createNode({
    name,
    install: { type: 'docker' },
    prepare: { type: 'local', reuseWorkingDirIfExists: false },
  })
}

export const createTestNodeLocal = (
  nodeName: string,
  reuseWorkingDirIfExists = false,
): Promise<ActyxNode> => {
  const prefix = 'test-node-local-linux'
  const name = `${prefix}-${nodeName}`
  return createNode({
    name,
    install: { type: 'linux' },
    prepare: { type: 'local', reuseWorkingDirIfExists },
  })
}
