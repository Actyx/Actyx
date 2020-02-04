import * as fetchMock from 'fetch-mock'
import { right } from 'fp-ts/lib/Either'
import { Observable } from 'rxjs'
import { FishName, Target } from '..'
import { CommandApi, UnsafeAsync } from '../commandApi'
import { timerFishType } from '../timerFish.support.test'
import { CommandExecutor } from './commandExecutor'

const cid1 = 'zdpuB2pwLskBDu5PZE2sepLyc3SRFPFgVXmnpzXVtWgam25kY'
const cid2 = 'zdpuAkXqnFs19s2c7C57zFhamkUac5csWpLtNi4RkDaxNJcgB'
const cid3 = 'zdpuAxU3nzaubvhMC8i5wBj2dmQbD5yBvjLpcswEENqpLvw1n'
const cid4 = 'zdpuAsUZXk73FTSkQdyevpB9WxdGmnu9GdCxAFQseZdHhWbVn'
const cid5 = 'zdpuAqHUzBEfYBTcsTjDKUMhLPgh1jtsnDDTd3KH4DagM5Jnz'
const prefix = 'http://localhost:5001/api/v0/block/get?arg='

const delay: () => Promise<void> = () => new Promise(res => setTimeout(res, 3000))

beforeEach(() => {
  const b1: fetchMock.MockResponseObject = {
    body: Buffer.from('+z/wAAAAAAAA', 'base64'),
  }
  const b2: fetchMock.MockResponseObject = {
    body: Buffer.from('+0AAAAAAAAAA', 'base64'),
  }

  fetchMock.get(prefix + cid1, b1, { sendAsJson: false })
  fetchMock.get(prefix + cid2, b2, { sendAsJson: false })
  fetchMock.get(prefix + cid3, 404)
  fetchMock.mock(prefix + cid4, () => Promise.reject('Server not found'))
  fetchMock.mock(prefix + cid5, () => delay().then(() => 'slow'))
})

const { http, pond } = CommandApi

describe('the command executor', () => {
  const executor = CommandExecutor()

  describe('the pond api', () => {
    it('should allow sending commands', done => {
      const executor1 = CommandExecutor({
        sendCommand: x => {
          expect(x.command).toEqual({ type: 'enable' })
          done()
        },
      })
      const target = Target.of(timerFishType, FishName.of('boo'))
      const ast = pond.send(target)({ type: 'enable' })
      return executor1(ast)
    })
    it('should fail when getting a non-existing state', () => {
      const ast = pond.peek(timerFishType, FishName.of('idonotexist'))
      // with a real pond this will never throw, since a non-existing fish will
      // be created.
      expect(() => executor(ast)).toThrow()
    })
  })

  describe('the http api', () => {
    it('should allow testing http get', () => {
      fetchMock.get('http://actyx.io?pointyhairedboss=true', 'awesome')
      const ast = http.request({
        method: 'GET',
        url: 'http://actyx.io',
        params: { pointyhairedboss: true },
        responseType: 'txt',
      })
      return executor(ast).then(res => expect(res).toEqual(right('awesome')))
    })
    it('should allow testing failing http get', () => {
      fetchMock.get('http://www.google.com', 404)
      const ast = http.get('http://www.google.com')
      return executor(ast).then(res => expect(res.isLeft()).toBeTruthy())
    })
    it('should allow testing http post', () => {
      fetchMock.post(
        'http://actyx.io/api/v1/createCustomer?iris=true&eos=true',
        '"customer created"',
      )
      const ast = http.request({
        method: 'POST',
        url: 'http://actyx.io/api/v1/createCustomer',
        data: { name: 'megaslime corporation' },
        params: { iris: true, eos: true },
      })
      return executor(ast).then(res => expect(res).toEqual(right('customer created')))
    })
    it('should allow testing failing http post', () => {
      fetchMock.post('http://actyx.io/api/v1/createCustomer', 500)
      const ast = http.request({
        method: 'POST',
        url: 'http://actyx.io/api/v1/createCustomer',
        data: { name: 'alphabet' },
      })
      return executor(ast).then(res => expect(res.isLeft()).toBeTruthy())
    })
  })
  it('should allow chaining requests', () => {
    const ast = CommandApi.of(right(1)).chain(a =>
      CommandApi.of(right(2)).chain(b => CommandApi.of([a, b])),
    )
    return expect(executor(ast)).resolves.toEqual([right(1), right(2)])
  })

  it('should allow executing complex asts', () => {
    const ast = CommandApi.all([CommandApi.of(right(1)), CommandApi.of(right(2))]).map(([a, b]) => {
      return { state: [a, b] }
    })
    return expect(executor(ast)).resolves.toEqual({ state: [right(1), right(2)] })
  })

  it('should allow logging', done => {
    const ast = CommandApi.of('x').log(text => {
      expect(text).toEqual('x')
      done()
    })
    return executor(ast)
  })

  describe('UnsafeAsync', () => {
    it('should allow asynchronous calculations', () => {
      const ast = UnsafeAsync(Observable.of('x').delay(10))
      return expect(executor(ast)).resolves.toEqual('x')
    })
    it('should only use the first result and not wait for more', () => {
      const ast = UnsafeAsync(Observable.of('x').concat(Observable.never()))
      return expect(executor(ast)).resolves.toEqual('x')
    })
  })
})

afterEach(() => fetchMock.restore())
