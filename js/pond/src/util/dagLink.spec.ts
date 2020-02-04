import { Link } from './dagLink'

describe('dagLink', () => {
  it('of', () => {
    expect(Link.of('abc')).toEqual({ ['/']: 'abc' })
  })
  it('maybe', () => {
    expect(Link.maybe('abc')).toEqual({ ['/']: 'abc' })
    expect(Link.maybe(undefined)).toEqual(undefined)
  })
  it('isLink', () => {
    const aLink = { ['/']: 'abc' }
    const noLink1 = { ['/']: null }
    const noLink2 = { ['//']: null }
    expect(Link.isLink(aLink)).toBeTruthy()
    expect(Link.isLink(noLink1)).toBeFalsy()
    expect(Link.isLink(noLink2)).toBeFalsy()
  })
  it('asLink', () => {
    const aLink = { ['/']: 'abc' }
    const noLink1 = { ['/']: null }
    const noLink2 = { ['//']: null }
    expect(Link.asLink(aLink)).toEqual(Link.of('abc'))
    expect(Link.asLink(noLink1)).toEqual(undefined)
    expect(Link.asLink(noLink2)).toEqual(undefined)
  })
  it('getCid', () => {
    expect(Link.getCid({ ['/']: 'abc' })).toEqual('abc')
  })
})
