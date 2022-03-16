/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { Predicate } from 'fp-ts/lib/Predicate'

export function collectFirst<T, U>(
  arr: T[],
  selector: (x: T) => U | null | undefined,
): U | undefined {
  let u
  arr.find((e) => {
    const cand = selector(e)
    if (cand !== undefined && cand !== null) {
      u = cand
      return true
    }
    return false
  })
  return u
}

export function collect<T, U>(
  arr: ReadonlyArray<T>,
  selector: (x: T) => U | null | undefined,
): U[] {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const u: any[] = []
  arr.forEach((e) => {
    const cand = selector(e)
    if (cand !== undefined && cand !== null) u.push(cand)
  })
  return u
}

export const shuffle = <T>(a: ReadonlyArray<T>): T[] => {
  const aa = [...a]
  let x: T
  for (let i = aa.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    x = aa[i]
    aa[i] = aa[j]
    aa[j] = x
  }
  return aa
}

export const permute = <T>(array: ReadonlyArray<T>): T[][] => {
  const result: T[][] = []
  const permute0 = (a: ReadonlyArray<T>, m: T[] = []) => {
    if (a.length === 0) {
      result.push(m)
    } else {
      for (let i = 0; i < a.length; i++) {
        const curr = a.slice()
        const next = curr.splice(i, 1)
        permute0(curr.slice(), m.concat(next))
      }
    }
  }
  permute0(array)
  return result
}

/**
 * Split an array into "runs" by a condition for consecutive elements.
 * Elements will be split when the condition returns true.
 *
 * For an empty array, this will return an empty array of runs. Each run is therefore at least one element long.
 */
export const split = <T>(
  a: ReadonlyArray<T>,
  cond: (x0: T, x1: T) => boolean,
): ReadonlyArray<ReadonlyArray<T>> => {
  if (a.length === 0) {
    return []
  }
  if (a.length === 1) {
    return [a]
  }
  const res: T[][] = []
  let j = 0
  for (let i = 1; i < a.length; i++) {
    const x0 = a[i - 1]
    const x1 = a[i]
    if (cond(x0, x1)) {
      res.push(a.slice(j, i))
      j = i
    }
  }
  if (j < a.length) {
    res.push(a.slice(j, a.length))
  }
  return res
}

/**
 * Apply a filter predicate on an array in place. The matching elements will be retained,
 * non-matching elements will be returned as a new array.
 */
export const retainInPlaceAndGetRemoved = <T>(a: T[], pred: Predicate<T>): T[] => {
  let i = 0
  let j = 0
  const result: T[] = []

  while (i < a.length) {
    const val = a[i]
    if (pred(val)) {
      a[j++] = val
    } else {
      result.push(val)
    }
    i++
  }

  a.length = j
  return result
}
/**
 * And-combine an array of predicates.
 * The empty array yields a predicate that is always true.
 */
export const andCombine = <T>(predicates: ReadonlyArray<Predicate<T>>) => {
  if (predicates.length === 0) {
    return () => true
  } else if (predicates.length === 1) {
    return predicates[0]
  } else {
    return (x: T) => {
      for (let i = 0; i < predicates.length; i++) {
        if (!predicates[i](x)) {
          return false
        }
      }
      return true
    }
  }
}

/**
 * Randomly interleaves several arrays so that the order within each array is preserved.
 */
export const interleaveRandom = <T>(arrays: ReadonlyArray<ReadonlyArray<T>>): T[] => {
  const length = arrays.reduce((acc, a) => acc + a.length, 0)
  const result: T[] = new Array(length)

  const nonEmpty = arrays.filter((x) => x.length > 0)
  const offsets = new Array(nonEmpty.length).fill(0)
  for (let i = 0; i < length; i++) {
    const pick = Math.floor(Math.random() * nonEmpty.length)
    result[i] = nonEmpty[pick][offsets[pick]]
    if (offsets[pick] + 1 === nonEmpty[pick].length) {
      nonEmpty.splice(pick, 1)
      offsets.splice(pick, 1)
    } else {
      offsets[pick]++
    }
  }

  return result
}
