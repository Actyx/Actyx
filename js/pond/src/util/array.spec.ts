/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { split } from './array'

describe('array', () => {
  describe('split', () => {
    const cond = (x0: number, x1: number): boolean => x0 + 1 !== x1
    it('should properly handle corner cases', () => {
      expect(split([], cond)).toEqual([])
      expect(split([1], cond)).toEqual([[1]])
    })
    it('should properly handle corner cases', () => {
      expect(split([], cond)).toEqual([])
      expect(split([1], cond)).toEqual([[1]])
    })
    it('should not split contiguous sequence', () => {
      expect(split([1, 2, 3, 4], cond)).toEqual([[1, 2, 3, 4]])
    })
    it('should split at the right places', () => {
      expect(split([1, 2, 4, 5], cond)).toEqual([[1, 2], [4, 5]])
      expect(split([1, 3, 4, 5], cond)).toEqual([[1], [3, 4, 5]])
      expect(split([1, 2, 3, 5], cond)).toEqual([[1, 2, 3], [5]])
      expect(split([0, 0, 0, 0], cond)).toEqual([[0], [0], [0], [0]])
      expect(split([1, 2, 4, 5, 7, 8], cond)).toEqual([[1, 2], [4, 5], [7, 8]])
    })
  })
})
