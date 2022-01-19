import { Person as P } from './types'

describe(`person type`, () => {
  it(`correctly compares two persons`, () => {
    expect(P.compare(P.of('a', '0'), P.of('b', '0'))).toBeLessThan(0)
    expect(P.compare(P.of('b', '0'), P.of('a', '0'))).toBeGreaterThan(0)
    expect(P.compare(P.of('a', 'c'), P.of('a', 'd'))).toBeLessThan(0)
    expect(P.compare(P.of('a', 'd'), P.of('a', 'c'))).toBeGreaterThan(0)
    expect(P.compare(P.of('a', 'a'), P.of('a', 'a'))).toBe(0)
    expect(P.compare(P.of('a', ''), P.of('a', ''))).toBe(0)
    expect(P.compare(P.of('', 'a'), P.of('', 'a'))).toBe(0)
    expect(P.compare(P.of('', ''), P.of('', ''))).toBe(0)
  })
  it(`correctly equates two persons`, () => {
    expect(P.equals(P.of('a', ''), P.of('a', ''))).toBe(true)
    expect(P.equals(P.of('', 'a'), P.of('', 'a'))).toBe(true)
    expect(P.equals(P.of('', ''), P.of('', ''))).toBe(true)
  })
  it(`correctly adds friends`, () => {
    expect(P.hasFriend(P.addFriend(P.of('', ''), 'f'), 'f')).toBe(true)
    expect(P.hasFriend(P.addFriend(P.of('', ''), 'f'), 'g')).toBe(false)
  })
  it(`doesn't add friends it already has`, () => {
    expect(P.addFriend(P.addFriend(P.of('', ''), 'f'), 'f').friendIds).toStrictEqual(['f'])
  })
  it(`doesn't throw if asked to remove a friend it doesn't have`, () => {
    expect(() => P.removeFriend(P.of('', ''), 'a')).not.toThrow()
  })
})
