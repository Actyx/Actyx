/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
declare module 'cids'
declare module 'format-util' {
  // eslint-disable-next-line
  declare var format: (msg: string, ...args: any[]) => string
  export = format
  export as namespace format
}
