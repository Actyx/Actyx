/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
export const assert: (condition: boolean, msg?: string) => void = (cond, msg) => {
  if (!cond) {
    throw new Error(msg || 'assertion failed')
  }
}

export function assertOrdered<T>(elems: T[], cond: (a: T, b: T) => number): void {
  for (let i = 0, length = elems.length; i < length - 1; i += 1) {
    if (cond(elems[i], elems[i + 1]) >= 0) {
      throw new Error(
        `unordered :${[elems[i], elems[i + 1]].map(x => JSON.stringify(x)).join(',')}`,
      )
    }
  }
}

export function assertDistinct<T>(elems: T[], proj: (value: T) => string): void {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const set: any = {}

  for (let i = 0, length = elems.length; i < length; i += 1) {
    const key = proj(elems[i])
    if (set[key]) {
      throw new Error(`not distinct ${key}`)
    }
    set[key] = true
  }
}
