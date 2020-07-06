/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function deepFreeze<T extends any>(o: T): T {
  Object.freeze(o)

  // Broken on TS 3.9

  return o
}
