import { nodeAddrValid } from '../common/util'

test('node address validation works', () => {
    expect(nodeAddrValid('google')).toBeFalsy()
    expect(nodeAddrValid('google.com')).toBeTruthy()
    expect(nodeAddrValid('some.google.com')).toBeTruthy()
    expect(nodeAddrValid('some.google.com:32932')).toBeTruthy()
    expect(nodeAddrValid('some.google.com:329321')).toBeFalsy()
    expect(nodeAddrValid('some.other')).toBeTruthy()
    expect(nodeAddrValid('some.other.domain')).toBeTruthy()
    expect(nodeAddrValid('some.other.domain:32932')).toBeTruthy()
    expect(nodeAddrValid('some')).toBeFalsy()
    expect(nodeAddrValid('localhost')).toBeTruthy()
    expect(nodeAddrValid('localhost:1')).toBeTruthy()
    expect(nodeAddrValid('localhost:10000')).toBeTruthy()
    expect(nodeAddrValid('1.1.1.1')).toBeTruthy()
    expect(nodeAddrValid('1.1.1.1.1')).toBeFalsy()
    expect(nodeAddrValid('1111.1.1.1')).toBeFalsy()
    expect(nodeAddrValid('1.1.1.1:1')).toBeTruthy()
    expect(nodeAddrValid('1.1.1.1:10000')).toBeTruthy()
    expect(nodeAddrValid('1.1.1.1:100000')).toBeFalsy()
    expect(nodeAddrValid('localhost:100000')).toBeFalsy()
    expect(nodeAddrValid('some.other.domain:329321')).toBeFalsy()
});

test('node address validation has known bugs', () => {
    // These should actually fail, adding here to document
    expect(nodeAddrValid('260.1.1.1')).toBeTruthy()
    expect(nodeAddrValid('260.1.1.1:65536')).toBeTruthy()
})
