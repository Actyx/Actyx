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
import { OffsetMap, Event, Subscription } from '../../../types'
import { _mkRequestObject } from '../../../client/event-service/subscribe'
import { Client } from '../../../client'
import { MockLineStreamingRequestResult } from '../../../client/__mocks__/request'
import { mkEvent, mkStreamIdentifier } from '../../../util'

const exOffsetMap: OffsetMap = {
  source1: 11,
  source2: -1,
}

const exEmptyOffsetMap: OffsetMap = {}
const exSubscriptionWithOnlySourceId = { source: 'source1' }
const exSubscriptionWithOnlyName = { streamName: 'name1' }

test('_mkRequestObject test 1', () => {
  const sub: Subscription[] = [exSubscriptionWithOnlySourceId, exSubscriptionWithOnlyName]
  expect(_mkRequestObject(sub, exOffsetMap)).toStrictEqual({
    lowerBound: {
      source1: 11,
      source2: -1,
    },
    subscriptions: [
      {
        source: 'source1',
      },
      {
        name: 'name1',
      },
    ],
  })
})

test('_mkRequestObject test 2', () => {
  const sub: Subscription = { streamSemantics: undefined, streamName: undefined, source: undefined }
  expect(_mkRequestObject(sub, exEmptyOffsetMap)).toStrictEqual({
    subscriptions: [{}],
  })
})

test('_mkRequestObject test 3', () => {
  const sub: Subscription[] = [exSubscriptionWithOnlySourceId, exSubscriptionWithOnlyName]
  expect(_mkRequestObject(sub, exEmptyOffsetMap)).toStrictEqual({
    subscriptions: [
      {
        source: 'source1',
      },
      {
        name: 'name1',
      },
    ],
  })
})

test('_mkRequestObject test 4', () => {
  const sub: Subscription[] = []
  expect(_mkRequestObject(sub, {})).toStrictEqual({
    subscriptions: [],
  })
})

test('_mkRequestObject test 5', () => {
  const sub: Subscription[] = []
  expect(_mkRequestObject(sub)).toStrictEqual({
    subscriptions: [],
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

// Note: this test will end after two events since the mock request module
// will call onDone when all events have been processed
test('subscribe call on successful request', () => {
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
    },
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
  Client().eventService.subscribe({
    subscriptions: Subscription.everything(),
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

test('subscribe call failed HTTP request', () => {
  setMockResult(MockLineStreamingRequestResult.ErrorWithoutLines('some HTTP error'))
  Client().eventService.subscribe({
    subscriptions: Subscription.everything(),
    onEvent: () => {
      fail("didn't expect any events")
    },
    onDone: () => {
      fail("didn't expect onDone to be called")
    },
    onError: error => {
      expect(error).toStrictEqual('some HTTP error')
    },
  })
})
