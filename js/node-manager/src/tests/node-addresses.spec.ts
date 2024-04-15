import { nodeAddrValid } from '../common/util'

test('node address validation works', () => {
  expect(nodeAddrValid('google')).toBeTruthy() // previously falsy, but this is should be a valid hostname
  expect(nodeAddrValid('google.com')).toBeTruthy()
  expect(nodeAddrValid('some.google.com')).toBeTruthy()
  expect(nodeAddrValid('some.google.com:32932')).toBeTruthy()
  expect(nodeAddrValid('some.other')).toBeTruthy()
  expect(nodeAddrValid('some.other.domain')).toBeTruthy()
  expect(nodeAddrValid('some.other.domain:32932')).toBeTruthy()
  expect(nodeAddrValid('some')).toBeTruthy()
  expect(nodeAddrValid('localhost')).toBeTruthy()
  expect(nodeAddrValid('local_host')).toBeTruthy()
  expect(nodeAddrValid('localhost:1')).toBeTruthy()
  expect(nodeAddrValid('localhost:10000')).toBeTruthy()
  expect(nodeAddrValid('1.1.1.1')).toBeTruthy()
  expect(nodeAddrValid('1.1.1.1.1')).toBeFalsy()
  expect(nodeAddrValid('1.1.1.1:1')).toBeTruthy()
  expect(nodeAddrValid('1.1.1.1:10000')).toBeTruthy()
  // port out of bound
  expect(nodeAddrValid('some.google.com:329321')).toBeFalsy()
  expect(nodeAddrValid('1.1.1.1:100000')).toBeFalsy()
  expect(nodeAddrValid('localhost:100000')).toBeFalsy()
  expect(nodeAddrValid('some.other.domain:329321')).toBeFalsy()
  // ipv4 out of bound
  expect(nodeAddrValid('1111.1.1.1')).toBeFalsy()
  expect(nodeAddrValid('260.1.1.1')).toBeFalsy()
  expect(nodeAddrValid('260.1.1.1:65536')).toBeFalsy()
})
