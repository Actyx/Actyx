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
import { _mkRequestObject } from '../../../client/event-service/publish'
import { mkEventDraft } from '../../../util'
import { Client } from '../../../client'
import { MockRequestResult } from '../../../client/__mocks__/request'

test('_mkRequestObject test 1', () => {
  expect(_mkRequestObject([])).toStrictEqual({
    data: [],
  })
})

test('_mkRequestObject test 2', () => {
  expect(
    _mkRequestObject([
      mkEventDraft('mysemantics1', 'myname', { foo: 'bar' }),
      mkEventDraft('mysemantics2', 'myname', {}),
    ]),
  ).toStrictEqual({
    data: [
      {
        name: 'myname',
        payload: { foo: 'bar' },
        semantics: 'mysemantics1',
      },
      {
        name: 'myname',
        payload: {},
        semantics: 'mysemantics2',
      },
    ],
  })
})

jest.mock('../../../client/request')

const setMockResult = (res: MockRequestResult) => {
  require('../../../client/request').__setDoRequestMockResult(res)
}

const unsetMockResult = () => {
  require('../../../client/request').__unsetDoRequestMockResult()
}

afterEach(() => {
  unsetMockResult()
})

test('publish call on successful request', () => {
  setMockResult(MockRequestResult.SucceedWithEmptyResult())
  const eds = [mkEventDraft('semantics', 'name', { foo: 'bar' })]
  Client().eventService.publish({
    eventDrafts: eds,
    onDone: () => {
      expect(1).toStrictEqual(1)
    },
    onError: error => {
      fail(`unexpectedly got an error: ${error}`)
    },
  })
})

test('publish call failed HTTP request', () => {
  setMockResult(MockRequestResult.FailWithError('got some HTTP error'))
  const eds = [mkEventDraft('semantics', 'name', { foo: 'bar' })]
  Client().eventService.publish({
    eventDrafts: eds,
    onDone: () => {
      fail(`onDone was called even though an error ocurred`)
    },
    onError: error => {
      expect(error).toStrictEqual('got some HTTP error')
    },
  })
})
