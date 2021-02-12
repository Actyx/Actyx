import { Client } from '@actyx/os-sdk'
import { CLI } from './cli'
import { ActyxOSNode } from './infrastructure/types'
import { Arch, Host, OS } from '../jest/types'
import { currentAxBinary } from './infrastructure/settings'
import { MyGlobal, Stubs } from '../jest/setup'

const mkNodeStub = async (
  axBinaryPath: string,
  os: OS,
  arch: Arch,
  host: Host,
  name: string,
  addr = 'localhost',
): Promise<ActyxOSNode> => {
  return {
    name,
    host,
    target: { os, arch, kind: { type: 'test' }, _private: { cleanup: () => Promise.resolve() } },
    ax: await CLI.build(addr, axBinaryPath),
    actyxOS: Client(),
    _private: {
      shutdown: () => Promise.resolve(),
      axBinaryPath: '',
      axHost: '',
      apiConsole: '',
      apiEvent: '',
      apiPond: '',
    },
  }
}

export const stubs = (<MyGlobal>global).stubs

// To be called in Jest's TestEnvironment prepration procedure: `environment.ts`
export const setupStubs = async (): Promise<Stubs> => {
  const axBinaryPath = await currentAxBinary()
  const def = await mkNodeStub(axBinaryPath, 'android', 'aarch64', 'android', 'foo', 'localhost')
  const unreachable = await mkNodeStub(
    axBinaryPath,
    'android',
    'aarch64',
    'android',
    'foo',
    '10.42.42.21',
  )

  return {
    axOnly: def,
    unreachable,
    mkStub: (a, b, c, d) => mkNodeStub(axBinaryPath, a, b, c, d, 'localhost'),
  }
}
