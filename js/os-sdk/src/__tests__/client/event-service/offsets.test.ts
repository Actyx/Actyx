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
import { Client } from '../../../client'
import '../../extensions.test'
import { MockRequestResult } from '../../../client/__mocks__/request'

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

test('offsets call on successful request', () => {
  setMockResult(MockRequestResult.SucceedWithResult(JSON.stringify({ source1: 10 })))
  Client().eventService.offsets({
    onOffsets: offsets => {
      expect(offsets).toStrictEqual({ source1: 10 })
    },
    onError: error => {
      fail(`unexpectedly got an error: ${error}`)
    },
  })
})

test('offsets call on failed HTTP request', () => {
  setMockResult(MockRequestResult.FailWithError('got some HTTP error'))
  Client().eventService.offsets({
    onOffsets: () => {
      fail(`unexpectedly got a result instead of an error`)
    },
    onError: error => {
      expect(error).toStrictEqual('got some HTTP error')
    },
  })
})

test('offsets call on successful request with non-json result', () => {
  setMockResult(MockRequestResult.SucceedWithResult('[['))
  Client().eventService.offsets({
    onOffsets: () => {
      fail('unexpectedly did not fail')
    },
    onError: error => {
      expect(error).toBeTruthy() // Expect some error content
    },
  })
})

test('offsets call on successful request with invalid result', () => {
  setMockResult(
    MockRequestResult.SucceedWithResult(
      JSON.stringify({
        source1: 'bar',
      }),
    ),
  )
  Client().eventService.offsets({
    onOffsets: () => {
      fail('unexpectedly did not fail')
    },
    onError: error => {
      expect(error).toBeTruthy() // Expect some error content
    },
  })
})
