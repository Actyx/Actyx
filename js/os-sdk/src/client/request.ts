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
import * as http from 'http'
import { RequestOptions } from 'http'
import { OnError, OnDone } from '../types'
import { StringDecoder } from 'string_decoder'
import { Transform } from 'stream'

/** @internal */
export interface RequestOpts {
  requestOptions: RequestOptions
  expectedStatusCode: number
  body?: string
  onResult: (res: string) => void
  onError?: OnError
}

/**
 * @internal
 * This function performs an HTTP request. It called onResult if the request
 * succeeds, meaning the status code is equal to the `expectedStatusCode` and no
 * other errors occur. Any error results in `onError` being called.
 *
 * @param opts Configuration of the request (see `RequestOpts`)
 */

export const doRequest = (opts: RequestOpts): void => {
  let chunks = ''
  let notified = false

  const req = http.request(opts.requestOptions, res => {
    if (res.statusCode === opts.expectedStatusCode) {
      res.setEncoding('utf8')
      res.on('data', chunk => {
        chunks += chunk
      })
      res.on('end', () => {
        if (!notified) {
          opts.onResult(chunks)
          notified = true
        }
      })
    } else {
      if (!notified) {
        opts.onError && opts.onError(`server returned unexpected code ${res.statusCode}`)
        notified = true
      }
      req.destroy()
    }
  })
  req.on('error', (err: string) => {
    if (!notified) {
      opts.onError && opts.onError(err)
      notified = true
    }
  })
  req.on('close', (err: string | undefined | null) => {
    req.destroy()
    if (!notified) {
      if (err !== '' && err !== undefined && err !== null) {
        opts.onError && opts.onError(err)
      } else {
        opts.onError &&
          opts.onError(`requested ended without sending a result and without an error`)
      }
      notified = true
    }
    req.destroy()
  })

  if (opts.body) {
    req.write(opts.body)
  }
  req.end()
}

/** @internal */
export interface LineStreamingRequestOpts {
  requestOptions: RequestOptions
  expectedStatusCode: number
  body?: string
  onLine: (line: string) => void
  onDone?: OnDone
  onError?: OnError
}

/**
 * @internal
 * This function performs a streaming (line-by-line) HTTP request. Whenever a
 * new, non-empty line is received, the `onLine` callback is called with that
 * line. If the request is ended using the returned function, the `onDone`
 * function is called. If an error occurs, the `onError` callback is called.
 *
 * @param opts Configuration of the request (see `LineStreamingRequestOpts`)
 * @returns A function that you should call to stop the request.
 */
export const doLineStreamingRequest = (opts: LineStreamingRequestOpts): (() => void) => {
  const utf8Decoder = new StringDecoder('utf8')

  let last = ''

  const subscriptionDecoder = new Transform({
    readableObjectMode: true,
    transform(chunk, _, cb) {
      let lines: string[] = []

      try {
        last += utf8Decoder.write(chunk)
        const list = last.split(/\r?\n/)
        const p = list.pop()
        last = p === undefined ? '' : p
        lines = list.filter(x => x.length > 0)
      } catch (err) {
        cb(err)
        return
      }

      if (lines.length > 0) {
        lines.forEach(l => this.push(l))
        cb(null)
      } else {
        cb()
      }
    },
  })

  let finished = false

  const req = http.request(opts.requestOptions, res => {
    if (res.statusCode === opts.expectedStatusCode) {
      res.pipe(subscriptionDecoder).on('data', str => {
        !finished && opts.onLine(str)
      })
    } else {
      if (!finished) {
        opts.onError && opts.onError(`server returned unexpected code ${res.statusCode}`)
        finished = true
      }
      req.destroy()
    }
  })

  const abortRequest = () => {
    req.destroy()
  }

  req.on('error', (err: string) => {
    if (!finished) {
      opts.onError && opts.onError(err)
      finished = true
    }
  })
  req.on('close', (err: string) => {
    req.destroy()
    if (!finished) {
      if (err !== '' && err !== undefined && err !== null) {
        opts.onError && opts.onError(err)
      } else {
        opts.onDone && opts.onDone()
      }
      finished = true
    }
  })

  if (opts.body) {
    req.write(opts.body)
  }
  req.end()

  return () => {
    abortRequest()
  }
}
