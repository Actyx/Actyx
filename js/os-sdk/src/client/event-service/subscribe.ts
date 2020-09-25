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
  OffsetMap,
  ApiClientOpts,
  Event,
  Subscription,
  SubscribeOpts,
  SubscribeStreamOpts,
} from '../../types'
import { mkSubscriptionApiObj, mkOffsetMapApiObj, tryMakeEventFromApiObj } from '../../util'
import * as uri from 'uri-js'
import * as http from 'http'

import * as CONSTANTS from '../constants'
import { doLineStreamingRequest } from '../request'
import { streamToEvents } from '../../util/decoding'

/** @internal */
export const _mkRequestObject = (
  subscriptions: Subscription | Subscription[],
  lowerBound?: OffsetMap,
): object => {
  const obj: { lowerBound?: object; subscriptions: object } = {
    subscriptions: {},
  }
  obj.subscriptions = mkSubscriptionApiObj(subscriptions)

  if (lowerBound && Object.keys(lowerBound).length > 0) {
    obj.lowerBound = mkOffsetMapApiObj(lowerBound)
  }

  return obj
}

/** @internal */
export const subscribe = (clientOpts: ApiClientOpts) => (opts: SubscribeOpts) => {
  // Note we add the extra slash here for safety
  const url =
    clientOpts.Endpoints.EventService.BaseUrl + clientOpts.Endpoints.EventService.Subscribe
  const { host, port, path } = uri.parse(url)

  // Request options
  const requestOptions = {
    hostname: host,
    port: port,
    path: path,
    method: 'POST',
    headers: CONSTANTS.CONTENT_TYPE_JSON_HEADER,
  }

  const body = JSON.stringify(_mkRequestObject(opts.subscriptions, opts.lowerBound))

  return doLineStreamingRequest({
    requestOptions,
    expectedStatusCode: 200,
    body,
    onLine: line => {
      let obj = {}
      try {
        obj = JSON.parse(line)
      } catch (err) {
        /* istanbul ignore next */
        // tslint:disable-next-line:no-string-throw
        throw `unable to parse line '${line}' as JSON`
      }

      const eitherEvent = tryMakeEventFromApiObj(obj)
      if (typeof eitherEvent === 'string') {
        /* istanbul ignore next */
        // tslint:disable-next-line:no-string-throw
        throw `unable to parse event: ${eitherEvent}`
      }

      opts.onEvent(eitherEvent as Event)
    },
    onDone: opts.onDone,
    onError: opts.onError,
  })
}

/** @internal */
export const subscribeStream = (clientOpts: ApiClientOpts) => (opts: SubscribeStreamOpts) => {
  const url =
    clientOpts.Endpoints.EventService.BaseUrl + clientOpts.Endpoints.EventService.Subscribe
  const { host, port, path } = uri.parse(url)

  // Request options
  const requestOptions = {
    hostname: host,
    port: port,
    path: path,
    method: 'POST',
    headers: CONSTANTS.CONTENT_TYPE_JSON_HEADER,
  }

  const body = JSON.stringify(_mkRequestObject(opts.subscriptions, opts.lowerBound))

  return new Promise<AsyncIterable<Event> & { cancel: () => void }>((res, rej) => {
    const req = http.request(requestOptions, msg => {
      if (msg.statusCode !== 200) {
        rej(new Error(`server responded with code ${msg.statusCode}`))
        req.destroy()
        return
      }

      res(Object.assign(streamToEvents(msg), { cancel: () => msg.destroy() }))
    })

    req.on('error', rej)
    req.on('close', rej)

    req.write(body)
    req.end()
  })
}
