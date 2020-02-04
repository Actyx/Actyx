import { Generator } from 'testcheck'
/*
 * This module has the purpose of exporting the `check` object installed
 * sneakily by jasmine-check. It also makes the `it` function available with proper types.
 */

require('jasmine-check').install()

export type Options = { times?: number; maxSize?: number; seed?: number }
/**
 * NOTE: These tests don't support async/await style! Save yourself the hassle.
 */
export type Check = {
  it1: <T1>(s: string, g1: Generator<T1>, f: (x: T1) => void) => void
  it1o: <T1>(s: string, options: Options, g: Generator<T1>, f: (x: T1) => void) => void
  it2: <T1, T2>(s: string, g: Generator<T1>, g2: Generator<T2>, f: (x: T1, y: T2) => void) => void
  it2o: <T1, T2>(
    s: string,
    options: Options,
    g: Generator<T1>,
    g2: Generator<T2>,
    f: (x: T1, y: T2) => void,
  ) => void
  it3: <T1, T2, T3>(
    s: string,
    g: Generator<T1>,
    g2: Generator<T2>,
    g3: Generator<T3>,
    f: (x: T1, y: T2, z: T3) => void,
  ) => void
  it4: <T1, T2, T3, T4>(
    s: string,
    g: Generator<T1>,
    g2: Generator<T2>,
    g3: Generator<T3>,
    g4: Generator<T4>,
    f: (a: T1, b: T2, c: T3, d: T4) => void,
  ) => void
  it5: <T1, T2, T3, T4, T5>(
    s: string,
    g1: Generator<T1>,
    g2: Generator<T2>,
    g3: Generator<T3>,
    g4: Generator<T4>,
    g5: Generator<T5>,
    f: (a: T1, b: T2, c: T3, d: T4, e: T5) => void,
  ) => void
  it6: <T1, T2, T3, T4, T5, T6>(
    s: string,
    g1: Generator<T1>,
    g2: Generator<T2>,
    g3: Generator<T3>,
    g4: Generator<T4>,
    g5: Generator<T5>,
    g6: Generator<T6>,
    f: (a: T1, b: T2, c: T3, d: T4, e: T5, f: T6) => void,
  ) => void
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const it = (global as any).check.it

export const Check: Check = {
  it1: (s, g1, f) => it(s, g1, f),
  it1o: (s, o, g1, f) => it(s, o, g1, f),
  it2: (s, g1, g2, f) => it(s, g1, g2, f),
  it2o: (s, o, g1, g2, f) => it(s, o, g1, g2, f),
  it3: (s, g1, g2, g3, f) => it(s, g1, g2, g3, f),
  it4: (s, g1, g2, g3, g4, f) => it(s, g1, g2, g3, g4, f),
  it5: (s, g1, g2, g3, g4, g5, f) => it(s, g1, g2, g3, g4, g5, f),
  it6: (s, g1, g2, g3, g4, g5, g6, f) => it(s, g1, g2, g3, g4, g5, g6, f),
}
