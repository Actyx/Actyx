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
  mkStreamIdentifier,
  mkEvent,
  mkOffsetMapApiObj,
  mkSubscriptionApiObj,
  orderingToApiStr,
} from '../../util'
import { OffsetMap, Ordering, Subscription } from '../../types'

test('mkStreamIdentifier works', () => {
  expect(mkStreamIdentifier('semantics1', 'name1', 'source1')).toStrictEqual({
    source: 'source1',
    streamName: 'name1',
    streamSemantics: 'semantics1',
  })
})

test('mkStreamIdentifier works', () => {
  const oSi = mkStreamIdentifier('semantics1', 'name1', 'source1')
  expect(mkEvent(oSi, 1000, 2000, 3000, { foo: 'bar' })).toStrictEqual({
    stream: {
      source: 'source1',
      streamName: 'name1',
      streamSemantics: 'semantics1',
    },
    timestamp: 1000,
    lamport: 2000,
    offset: 3000,
    payload: {
      foo: 'bar',
    },
  })
})

test('mkOffsetMapApiObject', () => {
  const exOffsetMap: OffsetMap = {
    source1: 11,
    source2: -1,
  }

  expect(mkOffsetMapApiObj(exOffsetMap)).toStrictEqual({
    source1: 11,
    source2: -1,
  })
})

test('mkOffsetMapApiObject with empty offset map', () => {
  const emptyOffsetMap: OffsetMap = {}

  expect(mkOffsetMapApiObj(emptyOffsetMap)).toStrictEqual({})
})

const exSubscriptionWithOnlySourceId = { source: 'source1' }
const exSubscriptionWithOnlyName = { streamName: 'name1' }
const exSubscriptionWithAllSet = {
  streamSemantics: 'semantics1',
  streamName: 'name1',
  source: 'source1',
}

test('mkSubscriptionApiObj works with filters 1', () => {
  const sub: Subscription[] = [exSubscriptionWithOnlySourceId, exSubscriptionWithOnlyName]
  expect(mkSubscriptionApiObj(sub)).toStrictEqual([{ source: 'source1' }, { name: 'name1' }])
})

test('mkSubscriptionApiObj works with filters 2', () => {
  const sub: Subscription[] = [exSubscriptionWithOnlyName]
  expect(mkSubscriptionApiObj(sub)).toStrictEqual([{ name: 'name1' }])
})

test('mkSubscriptionApiObj works with filters 3', () => {
  const sub: Subscription[] = [exSubscriptionWithAllSet]
  expect(mkSubscriptionApiObj(sub)).toStrictEqual([
    { source: 'source1', name: 'name1', semantics: 'semantics1' },
  ])
})

test('mkSubscriptionApiObj works with all none filters', () => {
  const sub: Subscription = { streamSemantics: undefined, streamName: undefined, source: undefined }
  expect(mkSubscriptionApiObj(sub)).toStrictEqual([{}])
})

test('orderingToApiStr', () => {
  expect(orderingToApiStr(Ordering.Lamport)).toBe('lamport')
  expect(orderingToApiStr(Ordering.LamportReverse)).toBe('lamport-reverse')
  expect(orderingToApiStr(Ordering.SourceOrdered)).toBe('source-ordered')
})
