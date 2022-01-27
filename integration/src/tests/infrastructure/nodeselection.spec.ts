/**
 * @jest-environment ./dist/jest/environment
 */
import { selectNodes } from '../../infrastructure/nodeselection'
import { ActyxNode } from '../../infrastructure/types'
import { MyGlobal } from '../../jest/setup'
import { mkNodeStub } from '../../stubs'

console.log(
  global.process.pid,
  (<MyGlobal>global).axNodeSetup.nodes[0].target,
  (<MyGlobal>global).isSuite,
)

let n1: ActyxNode
let n2: ActyxNode
let n3: ActyxNode
beforeAll(async () => {
  n1 = await mkNodeStub('android', 'aarch64', 'android', 'n0')
  n2 = await mkNodeStub('linux', 'x86_64', 'docker', 'n1')
  n3 = await mkNodeStub('windows', 'aarch64', 'process', 'n2')
})

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
