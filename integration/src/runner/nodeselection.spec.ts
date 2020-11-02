import { mkNodeStub } from '../stubs'
import { selectNodes } from './nodeselection'

const n1 = mkNodeStub('android', 'aarch64', 'android', ['webview'], 'n0')
const n2 = mkNodeStub('linux', 'x86_64', 'docker', ['docker'], 'n1')
const n3 = mkNodeStub('win', 'aarch64', 'process', [], 'n2')

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
