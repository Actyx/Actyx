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
/* tslint:disable:variable-name */
import { RequestOpts, LineStreamingRequestOpts } from '../request'

export interface MockRequestResult {
  result: string | null
  callOnErrorWith: string | null
}

export const MockRequestResult = {
  SucceedWithEmptyResult: (): MockRequestResult => ({
    result: '',
    callOnErrorWith: null,
  }),
  SucceedWithResult: (result: string): MockRequestResult => ({
    result,
    callOnErrorWith: null,
  }),
  FailWithError: (error: string): MockRequestResult => ({
    result: null,
    callOnErrorWith: error,
  }),
}

let __doRequestMockResult: MockRequestResult | null = null

export const __setDoRequestMockResult = (res: MockRequestResult) => {
  __doRequestMockResult = res
}

export const __unsetDoRequestMockResult = () => {
  __doRequestMockResult = null
}

export const doRequest = (opts: RequestOpts): void => {
  if (__doRequestMockResult === null) {
    throw new Error(`cannot mock request since no mock result defined`)
  }
  if (__doRequestMockResult.result !== null) {
    opts.onResult(__doRequestMockResult.result)
  }

  if (__doRequestMockResult.callOnErrorWith !== null) {
    if (opts.onError) {
      opts.onError(__doRequestMockResult.callOnErrorWith)
    }
  }
}

// This specifies the result of a line streaming request
export interface MockLineStreamingRequestResult {
  lines: string[]
  callOnErrorWith: string | null
  callOnDone: boolean
}

export const MockLineStreamingRequestResult = {
  NeverEnd: (lines: string[]): MockLineStreamingRequestResult => ({
    lines,
    callOnErrorWith: null,
    callOnDone: false,
  }),
  CloseAfterLines: (lines: string[]): MockLineStreamingRequestResult => ({
    lines,
    callOnErrorWith: null,
    callOnDone: true,
  }),
  ErrorAfterLines: (lines: string[], error: string): MockLineStreamingRequestResult => ({
    lines,
    callOnErrorWith: error,
    callOnDone: false,
  }),
  ErrorWithoutLines: (error: string): MockLineStreamingRequestResult => ({
    lines: [],
    callOnErrorWith: error,
    callOnDone: false,
  }),
}

let __doLineStreamingRequestMockResult: MockLineStreamingRequestResult | null = null

export const __setDoLineStreamingRequestMockResult = (
  mockResult: MockLineStreamingRequestResult,
) => {
  __doLineStreamingRequestMockResult = mockResult
}

export const __unsetDoLineStreamingRequestMockResult = () => {
  __doLineStreamingRequestMockResult = null
}

export const doLineStreamingRequest = (opts: LineStreamingRequestOpts) => {
  if (__doLineStreamingRequestMockResult === null) {
    throw new Error(`cannot mock line streaming request since no mock result defined`)
  }
  // Map over all lines
  __doLineStreamingRequestMockResult.lines.map(line => {
    opts.onLine(line)
  })
  // Call onError if error provided
  if (__doLineStreamingRequestMockResult.callOnErrorWith !== null) {
    if (opts.onError) {
      opts.onError(__doLineStreamingRequestMockResult.callOnErrorWith)
    }
  }
  // Call onDone if asked to do so
  if (__doLineStreamingRequestMockResult.callOnDone) {
    if (opts.onDone) {
      opts.onDone()
    }
  }
}

/* eslint-disable-next-line @typescript-eslint/no-explicit-any */
const request: any = jest.genMockFromModule('../request')
request.__setDoRequestMockResult = __setDoRequestMockResult
request.__unsetDoRequestMockResult = __unsetDoRequestMockResult
request.__setDoLineStreamingRequestMockResult = __setDoLineStreamingRequestMockResult
request.__unsetDoLineStreamingRequestMockResult = __unsetDoLineStreamingRequestMockResult
