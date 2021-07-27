import { Arch, Host, OS } from '../jest/types'
import { CLI } from './cli'
import { currentAxBinary } from './infrastructure/settings'
import { ActyxNode } from './infrastructure/types'

export const mkNodeStub = async (
  os: OS,
  arch: Arch,
  host: Host,
  name: string,
  addr = 'localhost',
): Promise<ActyxNode> => {
  const ax = await CLI.build(addr, await currentAxBinary())
  return {
    name,
    host,
    target: {
      os,
      arch,
      kind: { type: 'test' },
      execute: () => {
        throw new Error('stubs cannot execute')
      },
      _private: { cleanup: () => Promise.resolve() },
    },
    ax,
    _private: {
      shutdown: () => Promise.resolve(),
      actyxBinaryPath: '',
      workingDir: '',
      axBinaryPath: '',
      hostname: '',
      adminPort: 0,
      swarmPort: 0,
      apiPort: 0,
    },
  }
}

export const mkAx = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo').then((x) => x.ax)

export const mkAxWithUnreachableNode = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo', '10.42.42.21').then((x) => x.ax)
