import { Client } from '@actyx/os-sdk'
import { CLI } from './cli'
import { ActyxOSNode } from './infrastructure/types'
import { Arch, Host, OS, Runtime } from '../jest/types'
import { currentAxBinary } from './infrastructure/settings'

export const mkNodeStub = (
  os: OS,
  arch: Arch,
  host: Host,
  runtimes: Runtime[],
  name: string,
  addr = 'localhost',
): ActyxOSNode => {
  const axBinaryPath = currentAxBinary
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

export const stubNode = mkNodeStub('android', 'aarch64', 'android', ['webview'], 'foo', 'localhost')

export const stubNodeHostUnreachable = mkNodeStub(
  'android',
  'aarch64',
  'android',
  ['webview'],
  'foo',
  '123',
)

export const stubNodeActyxosUnreachable = mkNodeStub(
  'android',
  'aarch64',
  'android',
  ['webview'],
  'foo',
  'localhost:123',
)
