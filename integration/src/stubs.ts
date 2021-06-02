import { Arch, Host, OS } from '../jest/types'
import { CLI } from './cli'
import { currentAxBinary } from './infrastructure/settings'
import execa from 'execa'
import { ActyxNode } from './infrastructure/types'

export const mkNodeStub = (
  os: OS,
  arch: Arch,
  host: Host,
  name: string,
  addr = 'localhost',
): Promise<ActyxNode> =>
  currentAxBinary()
    .then((x) => CLI.build(addr, x))
    .then((ax) => ({
      name,
      host,
      target: {
        os,
        arch,
        kind: { type: 'test' },
        execute: () => execa.command('whoami'),
        _private: { cleanup: () => Promise.resolve() },
      },
      ax,
      _private: {
        shutdown: () => Promise.resolve(),
        actyxBinaryPath: '',
        axBinaryPath: '',
        axHost: '',
        httpApiOrigin: '',
        apiPond: '',
        apiSwarmPort: 0,
        apiEventsPort: 0,
      },
      startEphemeralNode: () => Promise.reject('Not supported on stubs'),
    }))

export const mkAx = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo').then((x) => x.ax)

export const mkAxWithUnreachableNode = (): Promise<CLI> =>
  mkNodeStub('android', 'aarch64', 'android', 'foo', '10.42.42.21').then((x) => x.ax)
