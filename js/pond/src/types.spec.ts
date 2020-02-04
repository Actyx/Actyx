import { gen } from 'testcheck'
import { Check } from './hackcheck'
import {
  isBoolean,
  isNumber,
  isString,
  Milliseconds,
  Semantics,
  SourceId,
  Timestamp,
} from './types'

describe('Semantics', () => {
  it('should not allow creating a non-jellyfish starting with jelly-', () => {
    expect(() => Semantics.of('jelly-foo')).toThrow()
  })
})

describe('SourceId.random', () => {
  it('must create a random SourceID', () => expect(SourceId.random(42)).toHaveLength(42))
})

describe('isString', () => {
  Check.it1('should return true for strings', gen.string, value => {
    expect(isString(value)).toBeTruthy()
  })

  Check.it1('should return false for numbers', gen.number, value => {
    expect(isString(value)).toBeFalsy()
  })

  it('should return false for booleans, undefined, objects, arrays and functions', () => {
    expect(isString(false)).toBeFalsy()
    expect(isString(true)).toBeFalsy()
    expect(isString(undefined)).toBeFalsy()
    expect(isString({})).toBeFalsy()
    expect(isString([])).toBeFalsy()
    expect(isString(() => ({}))).toBeFalsy()
  })
})

describe('isNumber', () => {
  Check.it1('should return false for strings', gen.string, value => {
    expect(isNumber(value)).toBeFalsy()
  })

  Check.it1('should return true for numbers', gen.number, value => {
    expect(isNumber(value)).toBeTruthy()
  })

  it('should return false for booleans, undefined, objects, arrays and functions', () => {
    expect(isNumber(false)).toBeFalsy()
    expect(isNumber(true)).toBeFalsy()
    expect(isNumber(undefined)).toBeFalsy()
    expect(isNumber({})).toBeFalsy()
    expect(isNumber([])).toBeFalsy()
    expect(isNumber(() => ({}))).toBeFalsy()
  })
})

describe('isBoolean', () => {
  Check.it1('should return false for strings', gen.string, value => {
    expect(isBoolean(value)).toBeFalsy()
  })

  Check.it1('should return false for numbers', gen.number, value => {
    expect(isBoolean(value)).toBeFalsy()
  })

  it('should return true for booleans and false for undefined, objects, arrays and functions', () => {
    expect(isBoolean(false)).toBeTruthy()
    expect(isBoolean(true)).toBeTruthy()
    expect(isBoolean(undefined)).toBeFalsy()
    expect(isBoolean({})).toBeFalsy()
    expect(isBoolean([])).toBeFalsy()
    expect(isBoolean(() => ({}))).toBeFalsy()
  })
})

describe('Timestamp', () => {
  const now = 1545056028065

  it('Timestamp.now()', () => expect(Timestamp.now(now)).toEqual(1545056028065000))

  it('Timestamp.toSeconds()', () => expect(Timestamp.toSeconds(Timestamp.of(3 * 1e6))).toEqual(3))

  it('Timestamp.toMilliseconds()', () =>
    expect(Timestamp.toMilliseconds(Timestamp.of(3 * 1e6))).toEqual(3000))

  it('Timestamp.fromSeconds()', () => expect(Timestamp.fromSeconds(3)).toEqual(3 * 1e6))

  it('Timestamp.fromMilliseconds()', () => expect(Timestamp.fromMilliseconds(3)).toEqual(3 * 1e3))
})

describe('Milliseconds', () => {
  const now = 1545056028065
  it('Timestamp.fromAnyToMillis()', () => {
    const now0 = new Date().valueOf()
    expect(Milliseconds.fromAny(now0 * 1e3)).toEqual(now0)
    expect(Milliseconds.fromAny(now0)).toEqual(now0)
    expect(Milliseconds.fromAny(now)).toEqual(now)
  })
})
