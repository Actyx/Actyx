/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
import { entriesOf, KeyValueMap, none, todo, toKeyValueMap, unreachable, valuesOf } from '.'

describe('todo', () => {
  it('must throw', () => {
    expect(() => todo()).toThrowError()
  })
})

describe('none', () => {
  it('must throw', () => {
    expect(() => none()).toThrowError()
  })
})

describe('unreachable', () => {
  it('must throw', () => {
    // $ExpectError
    expect(() => unreachable()).toThrow('Unreachable')
  })
})

describe('KeyValueMap', () => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Foo = KeyValueMap<any>
  const map: Foo = {
    one: 'first',
    two: 'second',
    three: undefined,
  }

  it('valuesOf() without undefined vals', () => {
    // type Foo = { [x: string]: any | undefined }
    const foo: Foo = { foo: 'foo', bar: 'bar', baz: false, qux: 0, quux: '' }
    const actual: ReadonlyArray<string> = valuesOf(foo)
    expect(actual).toEqual(['foo', 'bar', false, 0, ''])
  })

  it('valuesIf() with undefined vals', () => expect(valuesOf(map)).toEqual(['first', 'second']))

  it('entriesOf()', () =>
    expect(entriesOf(map)).toEqual([
      ['one', 'first'],
      ['two', 'second'],
    ]))

  it('toKeyValueMap()', () =>
    expect(
      toKeyValueMap('id', [
        { id: 1, v: '1' },
        { id: 2, v: '2' },
        { id: 3, v: '3' },
      ]),
    ).toEqual({
      '1': { id: 1, v: '1' },
      '2': { id: 2, v: '2' },
      '3': { id: 3, v: '3' },
    }))

  it('toKeyValueMap() complex', () =>
    expect(
      toKeyValueMap('id', [
        { id: 1, v: '1', a: { foo: 'boo' } },
        { id: 2, v: '2', b: { bar: 'baz' } },
        { id: 3, v: '3', c: null },
      ]),
    ).toEqual({
      '1': { id: 1, v: '1', a: { foo: 'boo' } },
      '2': { id: 2, v: '2', b: { bar: 'baz' } },
      '3': { id: 3, v: '3', c: null },
    }))
})
