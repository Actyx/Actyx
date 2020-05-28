/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
import {
  NonEmptyStringType,
  tryMakeEventFromApiObj,
  tryMakeOffsetMapFromApiObj,
  ZeroOrGreaterType,
} from '../../util/decoding'
import '../extensions.test'

test('NonEmptyStringType works', () => {
  expect(NonEmptyStringType.decode('asd')).isRight()
  expect(NonEmptyStringType.decode('')).isLeft()
  expect(NonEmptyStringType.decode(' ')).isLeft()
  expect(NonEmptyStringType.decode('   ')).isLeft()
  expect(NonEmptyStringType.decode(10)).isLeft()
  expect(NonEmptyStringType.decode({ foo: 'bar' })).isLeft()
  expect(NonEmptyStringType.is('bar')).toBe(true)
  expect(NonEmptyStringType.is('')).toBe(false)
})

test('ZeroOrGreater works', () => {
  expect(ZeroOrGreaterType.decode('')).isLeft()
  expect(ZeroOrGreaterType.decode(-1)).isLeft()
  expect(ZeroOrGreaterType.decode(0)).isRight()
  expect(ZeroOrGreaterType.decode(1)).isRight()
  expect(ZeroOrGreaterType.decode(10)).isRight()
})

test('tryMakeEventFromApiObj recognizes correct object', () => {
  expect(
    tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toStrictEqual({
    stream: {
      source: 'source1',
      streamName: 'name1',
      streamSemantics: 'semantics1',
    },
    timestamp: 10,
    lamport: 10,
    offset: 10,
    payload: { foo: 'bar' },
  })
})

test('tryMakeEventFromApiObj recognizes missing stream prop', () => {
  expect(typeof tryMakeEventFromApiObj({})).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped stream prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: 10,
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing stream.source prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped stream.source prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 10,
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes stream.source is empty string', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: '',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing stream.name prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped stream.name prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: {},
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes stream.name is empty string', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: '',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing stream.semantics prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped stream.semantics prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: {},
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes stream.semantics is empty string', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: '',
      },
      timestamp: 10,
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing timestamp prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped timestamp prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: [1, 2, 3],
      lamport: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing lamport prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped lamport prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: [1, 2, 3],
      offset: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes missing offset prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeEventFromApiObj recognizes mistyped offset prop', () => {
  expect(
    typeof tryMakeEventFromApiObj({
      stream: {
        source: 'source1',
        name: 'name1',
        semantics: 'semantics1',
      },
      timestamp: 10,
      lamport: 10,
      offset: '102',
      payload: { foo: 'bar' },
    }),
  ).toBe('string')
})

test('tryMakeOffsetMapFromApiObj works with empty map', () => {
  expect(tryMakeOffsetMapFromApiObj({})).toStrictEqual({})
})

test('tryMakeOffsetMapFromApiObj recognizes wrong property types', () => {
  expect(
    typeof tryMakeOffsetMapFromApiObj({
      source1: 100,
      source2: {},
    }),
  ).toBe('string')
  expect(
    typeof tryMakeOffsetMapFromApiObj({
      source1: 'asd',
      source2: 203,
    }),
  ).toBe('string')
  expect(
    typeof tryMakeOffsetMapFromApiObj({
      source0: {},
    }),
  ).toBe('string')
  expect(
    typeof tryMakeOffsetMapFromApiObj({
      source0: {},
      source1: null,
      source2: 10,
      source3: -10,
    }),
  ).toBe('string')
})

test('tryMakeOffsetMapFromApiObj recognizes offset < -1', () => {
  expect(
    typeof tryMakeOffsetMapFromApiObj({
      source0: -2,
    }),
  ).toBe('string')
})
