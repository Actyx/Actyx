import { assert, assertDistinct, assertOrdered, collect, collectFirst } from '.'

describe('collect', () => {
  it('must yield all matching elements', () => {
    const input = ['a', 'b1', 'c', 'd2']
    const output = collect(input, e => {
      const res = e.match(/([0-9]+)/)
      return res ? parseInt(res[1], 10) : undefined
    })
    expect(output).toEqual([1, 2])
  })
})

describe('collectFirst', () => {
  it('must yield the first matching element', () => {
    const input = ['a', 'b1', 'c', 'd2']
    const output = collectFirst(input, e => {
      const res = e.match(/([0-9]+)/)
      return res ? parseInt(res[1], 10) : undefined
    })
    expect(output).toEqual(1)
  })
})

describe('assert', () => {
  it('must return normally', () => {
    expect(assert(true, '')).toBeUndefined()
  })
  it('must throw', () => {
    expect(() => assert(false)).toThrowError()
  })
  it('must throw with message', () => {
    expect(() => assert(false, 'boo')).toThrowError('boo')
  })
})

describe('assertOrdered', () => {
  function cmp(x: number, y: number): number {
    if (x < y) return -1
    else if (x > y) return 1
    else return 0
  }
  it('must return normally', () => {
    expect(assertOrdered([0, 1, 2], cmp)).toBeUndefined()
  })
  it('must throw', () => {
    expect(() => assertOrdered([0, 2, 1], cmp)).toThrowError('unordered')
  })
})

describe('assertDistinct', () => {
  it('must return normally', () => {
    expect(assertDistinct(['0', '1', '2'], x => x)).toBeUndefined()
  })
  it('must throw', () => {
    expect(() => assertDistinct(['a', 'b', 'a'], x => x)).toThrowError()
  })
})
