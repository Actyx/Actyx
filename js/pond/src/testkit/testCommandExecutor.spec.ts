/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { right } from 'fp-ts/lib/Either'
import { FishName, Target } from '..'
import { CommandApi } from '../commandApi'
import { timerFishType } from '../timerFish.support.test'
import { TestCommandExecution, TestCommandExecutor } from './testCommandExecutor'

const httpGet = {
  'http://actyx.io?pointyhairedboss=true': 'awesome',
}
const httpPost = {
  'http://actyx.io/api/v1/createCustomer?iris=true&eos=true': right('customer created'),
}
const fishStates = {
  timerFish: { ithinkthereforeiam: { type: 'enabled' } },
}
const executor = TestCommandExecutor({ httpGet, httpPost, fishStates })

const { http, pond } = CommandApi

describe('the test command executor', () => {
  describe('the pond api', () => {
    it('should allow sending commands', () => {
      const target = Target.of(timerFishType, FishName.of('boo'))
      const ast = pond.send(target)({ type: 'enable' })
      expect(executor(ast).effects).toEqual(['pond.send(timerFish, boo)({"type":"enable"})'])
    })
    it('should fail when getting a non-existing state', () => {
      const ast = pond.peek(timerFishType, FishName.of('idonotexist'))
      expect(() => executor(ast)).toThrow()
    })
    it('should allow getting an existing state', () => {
      const ast = pond.peek(timerFishType, FishName.of('ithinkthereforeiam'))
      expect(executor(ast).result).toEqual({ type: 'enabled' })
    })
  })
  describe('the http api', () => {
    it('should allow testing http get', () => {
      const ast = http.request({
        method: 'GET',
        url: 'http://actyx.io',
        params: { pointyhairedboss: true },
      })
      expect(executor(ast).result).toEqual(right('awesome'))
    })
    it('should allow testing failing http get', () => {
      const ast = http.get('http://www.google.com')
      expect(executor(ast).result.isLeft()).toBeTruthy()
    })
    it('should allow testing http post', () => {
      const ast = http.request({
        method: 'POST',
        url: 'http://actyx.io/api/v1/createCustomer',
        data: { name: 'megaslime corporation' },
        params: { iris: true, eos: true },
      })
      expect(executor(ast).result).toEqual(right('customer created'))
    })
    it('should allow testing failing http post', () => {
      const ast = http.request({
        method: 'POST',
        url: 'http://actyx.io/api/v1/createCustomer',
        data: { name: 'alphabet' },
      })
      expect(executor(ast).result.isLeft()).toBeTruthy()
    })
  })
  describe('TestCommandExecution', () => {
    it('should fail when being used with sync command results', () => {
      const x = TestCommandExecution({ httpGet, httpPost, fishStates })
      expect(() => x([])).toThrow()
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect(() => x(undefined as any)).toThrow()
    })
  })
  it('should allow testing all and map', () => {
    const ast = CommandApi.all([
      CommandApi.of(right(1)),
      CommandApi.of(right(2)),
      CommandApi.of(right(3)),
    ]).map(([a, b, c]) => [c, b, a])
    const tr = executor(ast)
    expect(tr.result).toEqual([right(3), right(2), right(1)])
    expect(tr.effects.length).toEqual(0)
  })
  it('should allow testing log', () => {
    const ast = CommandApi.of('boo').log()
    const tr = executor(ast)
    expect(tr.result).toEqual('boo')
    expect(tr.effects.length).toEqual(1)
  })
  it('should allow testing chain', () => {
    const ast = CommandApi.of(right(1)).chain(r1 =>
      CommandApi.of(right(2)).map(r2 => `${r2} ${r1}`),
    )
    const tr = executor(ast)
    expect(tr.result).toEqual('right(2) right(1)')
    expect(tr.effects.length).toEqual(0)
  })
})
