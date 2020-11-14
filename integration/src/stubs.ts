import { Client } from '@actyx/os-sdk'
import settings from '../settings'

import { CLI } from './cli/cli'
import { ActyxOSNode, Arch, Host, OS, Runtime } from './runner/types'

export const mkNodeStub = (
  os: OS,
  arch: Arch,
  host: Host,
  runtimes: Runtime[],
  name: string,
  addr = 'localhost',
): ActyxOSNode => {
  const axBinaryPath = settings.binaryPath.ax
  return {
    name,
    host,
    runtimes,
    target: { os, arch, kind: { type: 'test' }, _private: { shutdown: () => Promise.resolve() } },
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
