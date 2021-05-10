/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { ord } from 'fp-ts'
import { mergeSortedInto } from '.'

describe('mergeSortedInto', () => {
  const merge = (l: number[], r: number[]): [number, number[]] => {
    const out = l.slice().concat(...r)
    const h = mergeSortedInto(l, r, out, ord.ordNumber.compare)
    return [h, out]
  }

  it('should sort without overlap', () => {
    expect(merge([1, 2, 3], [4, 5, 6])).toEqual([2, [1, 2, 3, 4, 5, 6]])
  })

  it('should sort with partial overlap', () => {
    expect(merge([1, 2, 3, 4], [4, 5, 6])).toEqual([3, [1, 2, 3, 4, 5, 6]])
  })

  it('should sort with exact overlap', () => {
    expect(merge([1, 2, 3, 4, 5, 6], [4, 5, 6])).toEqual([5, [1, 2, 3, 4, 5, 6]])
  })

  it('should sort with more overlap', () => {
    expect(merge([1, 2, 3, 4, 5, 6], [4, 5])).toEqual([5, [1, 2, 3, 4, 5, 6]])
  })
})
