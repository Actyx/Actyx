/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { sort } from 'fp-ts/lib/Array'
import { ordNumber } from 'fp-ts/lib/Ord'
import { gen } from 'testcheck'
import { Check } from '../hackcheck'
import { binarySearch, getInsertionIndex } from './binarySearch'

const N = 100
describe('binarySearch', () => {
  const check = (array: ReadonlyArray<number>) => {
    const bs = (e: number) => binarySearch(array, e, (a, b) => a - b)
    const results = Array.from(Array(N).keys()).map(bs)
    return expect(
      results.some(
        (n, i) =>
          (n >= 0 && array[n] !== i) ||
          (n < 0 && (-n - 1 < array.length && array[-n - 1] <= i)) ||
          (n < 0 && (-n - 2 >= 0 && array[-n - 2] >= i)),
      ),
    ).toBeFalsy()
  }
  it('should work with empty', () => check([]))
  it('should work with first', () => check([0]))
  it('should work with last', () => check([N - 1]))
  it('should work distinct', () => check([1, 2, 5, 9, 11, 12, 15, 20, 25, 40, 41, 41, 80]))
  it('should work with duplicates', () =>
    check([
      1,
      2,
      2,
      2,
      5,
      9,
      11,
      12,
      12,
      12,
      12,
      15,
      20,
      20,
      20,
      25,
      40,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      41,
      80,
    ]))
})

describe('getInsertionIndex', () => {
  const genSafeNumber = gen.numberWithin(Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER)

  Check.it2(
    'should work',
    gen.uniqueArray(genSafeNumber, { minSize: 1 }),
    genSafeNumber,
    (arr, e) => {
      const arr0 = sort(ordNumber)(arr)
      const idx = getInsertionIndex(arr0, e, (a, b) => a - b)
      if (idx > 0) {
        expect(arr0[idx - 1] <= e).toBeTruthy()
      } else {
        expect(arr0[0] >= e).toBeTruthy()
      }
    },
  )
})
