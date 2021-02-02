import { stubs } from '../stubs'
import { selectNodes } from './nodeselection'

const n1 = stubs.mkStub('android', 'aarch64', 'android', 'n0')
const n2 = stubs.mkStub('linux', 'x86_64', 'docker', 'n1')
const n3 = stubs.mkStub('windows', 'aarch64', 'process', 'n2')

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
