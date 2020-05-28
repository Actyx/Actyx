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
/* eslint-disable @typescript-eslint/no-empty-function */
import { _mkRequestObject } from '../../../client/event-service/query'
import { Ordering, Event, Subscription } from '../../../types'
import { Client } from '../../../client'
import { MockLineStreamingRequestResult } from '../../../client/__mocks__/request'
import { mkEvent, mkStreamIdentifier } from '../../../util'

const exSubscriptionWithOnlySourceId = { source: 'source1' }
const exSubscriptionWithOnlyName = { streamName: 'name1' }

test('_mkRequestObject test 1', () => {
  expect(
    _mkRequestObject(
      [exSubscriptionWithOnlyName, exSubscriptionWithOnlySourceId],
      Ordering.LamportReverse,
      { source3: 10000 },
      { source1: 10, source2: 3 },
    ),
  ).toStrictEqual({
    lowerBound: {
      source1: 10,
      source2: 3,
    },
    upperBound: {
      source3: 10000,
    },
    subscriptions: [{ name: 'name1' }, { source: 'source1' }],
    order: 'lamport-reverse',
  })
})

test('_mkRequestObject with empty upper bounds has empty upper bounds object in query payload', () => {
  expect(
    _mkRequestObject(
      [exSubscriptionWithOnlyName, exSubscriptionWithOnlySourceId],
      Ordering.LamportReverse,
      {}
    ),
  ).toStrictEqual({
    upperBound: {},
    subscriptions: [{ name: 'name1' }, { source: 'source1' }],
    order: 'lamport-reverse',
  })
})

jest.mock('../../../client/request')

const setMockResult = (res: MockLineStreamingRequestResult) => {
  require('../../../client/request').__setDoLineStreamingRequestMockResult(res)
}

const unsetMockResult = () => {
  require('../../../client/request').__unsetDoLineStreamingRequestMockResult()
}

afterEach(() => {
  unsetMockResult()
})

test('query call on successful request', () => {
  let eIx = -1

  const apiEvents = [
    {
      stream: {
        semantics: 'semantics1',
        name: 'name1',
        source: 'source1',
      },
      timestamp: 10,
      lamport: 20,
      offset: 30,
      payload: {
        foo: 'bar',
      },
    },
    {
      stream: {
        semantics: 'semantics2',
        name: 'name2',
        source: 'source2',
      },
      timestamp: 100,
      lamport: 200,
      offset: 300,
      payload: {
        bar: 'foo',
      },
    }
  ]

  const expectedEvents = [
    mkEvent(mkStreamIdentifier('semantics1', 'name1', 'source1'), 10, 20, 30, {
      foo: 'bar',
    }),
    mkEvent(mkStreamIdentifier('semantics2', 'name2', 'source2'), 100, 200, 300, {
      bar: 'foo',
    }),
  ]

  setMockResult(
    MockLineStreamingRequestResult.CloseAfterLines(apiEvents.map(e => JSON.stringify(e))),
  )
  const receivedEvents: Event[] = []
  Client().eventService.query({
    subscriptions: Subscription.everything(),
    ordering: Ordering.Lamport,
    upperBound: {},
    onEvent: event => {
      eIx += 1
      receivedEvents.push(event)
      const expectedEvent = expectedEvents[eIx]
      expect(event).toStrictEqual(expectedEvent)
    },
    onDone: () => {
      expect(receivedEvents.length).toBe(expectedEvents.length)
    },
    onError: error => {
      fail(`unexpectedly got error: ${error}`)
    },
  })
})

test('query call failed HTTP request', () => {
  setMockResult(MockLineStreamingRequestResult.ErrorWithoutLines('some HTTP error'))
  Client().eventService.query({
    subscriptions: Subscription.everything(),
    ordering: Ordering.Lamport,
    upperBound: {},
    onEvent: () => {
      fail("didn't expect any events")
    },
    onDone: () => {
      fail("didn't expect for onDone to be called")
    },
    onError: error => {
      expect(error).toStrictEqual('some HTTP error')
    },
  })
})
