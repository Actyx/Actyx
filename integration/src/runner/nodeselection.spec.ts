/* eslint-disable @typescript-eslint/no-empty-function */
import { CLI } from '../ax'
import { selectNodes } from './nodeselection'
import { ActyxOSNode, Arch, Host, OS, Runtime } from './types'

let counter = 0
const mkNode = (os: OS, arch: Arch, host: Host, runtimes: Runtime[]): ActyxOSNode => {
  counter += 1
  const name = `n${counter}`
  return {
    name,
    host,
    runtimes,
    target: { os, arch, kind: 'test', shutdown: () => {} },
    ax: new CLI('localhost'),
    shutdown: () => {},
  }
}

const n1 = mkNode('android', 'aarch64', 'android', ['webview'])
const n2 = mkNode('linux', 'x86_64', 'docker', ['docker'])
const n3 = mkNode('win', 'aarch64', 'process', [])

describe('NodeSelection', () => {
  it('should fail', () => {
    expect(selectNodes([{ os: 'linux' }], [])).toEqual(null)
  })
  it('should select single node', () => {
    expect(selectNodes([{ os: 'linux' }], [n1, n2, n3])).toEqual([n2])
  })
  it('should select multiple', () => {
    expect(selectNodes([{}, {}, {}], [n1, n2, n3])).toEqual([n1, n2, n3])
    expect(selectNodes([{}, {}, { host: 'process' }], [n1, n2, n3])).toEqual([n1, n2, n3])
  })
})
