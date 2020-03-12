/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { left, right } from 'fp-ts/lib/Either'
import { CE, CommandApi } from './commandApi'
import { TestCommandExecutor } from './testkit'

describe('CE', () => {
  it('liftC should lift a function that returns a CommandApi into CE', () => {
    const f = (x: number) => CommandApi.of(x * x)
    const ex = TestCommandExecutor({})
    expect(ex(CE.liftC(f)(left('error'))).result).toEqual(left('error'))
    expect(ex(CE.liftC(f)(right(2))).result).toEqual(right(4))
  })
  it('liftCE should lift a function that returns a CommandApi into CE', () => {
    const f = (x: number) => CommandApi.of(right(x * x))
    const ex = TestCommandExecutor({})
    expect(ex(CE.liftCE(f)(left('error'))).result).toEqual(left('error'))
    expect(ex(CE.liftCE(f)(right(2))).result).toEqual(right(4))
  })
})
