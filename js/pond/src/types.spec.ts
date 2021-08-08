/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Semantics } from './types'

describe('Semantics', () => {
  it('should not allow creating a non-jellyfish starting with jelly-', () => {
    expect(() => Semantics.of('jelly-foo')).toThrow()
  })
})
