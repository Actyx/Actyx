/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
// https://github.com/Microsoft/TypeScript/issues/4895#issuecomment-401067935
declare const OpaqueTagSymbol: unique symbol
declare class OpaqueTag<S extends symbol> {
  private [OpaqueTagSymbol]: S
}

export type Opaque<T, S extends symbol> = T & OpaqueTag<S>
