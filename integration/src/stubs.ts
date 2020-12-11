import { Client } from '@actyx/os-sdk'
import { CLI } from './cli'
import { ActyxOSNode } from './infrastructure/types'
import { Arch, Host, OS, Runtime } from '../jest/types'
import { currentAxBinary } from './infrastructure/settings'
import { MyGlobal, Stubs } from '../jest/setup'

const mkNodeStub = (
  axBinaryPath: string,
  os: OS,
  arch: Arch,
  host: Host,
  runtimes: Runtime[],
  name: string,
  addr = 'localhost',
): ActyxOSNode => {
  return {
    name,
    host,
    runtimes,
    target: { os, arch, kind: { type: 'test' }, _private: { cleanup: () => Promise.resolve() } },
    ax: new CLI(addr, axBinaryPath),
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
  const def = mkNodeStub(
    axBinaryPath,
    'android',
    'aarch64',
    'android',
    ['webview'],
    'foo',
    'localhost',
  )
  const hostUnreachable = mkNodeStub(
    axBinaryPath,
    'android',
    'aarch64',
    'android',
    ['webview'],
    'foo',
    'idontexist',
  )
  const actyxOSUnreachable = mkNodeStub(
    axBinaryPath,
    'android',
    'aarch64',
    'android',
    ['webview'],
    'foo',
    'localhost:123',
  )

  return {
    axOnly: def,
    hostUnreachable,
    actyxOSUnreachable,
    mkStub: (a, b, c, d, e) => mkNodeStub(axBinaryPath, a, b, c, d, e, 'localhost'),
  }
}
