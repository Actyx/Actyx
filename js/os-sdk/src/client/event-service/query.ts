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
import { OffsetMap, Ordering, ApiClientOpts, Subscription, Event, QueryOpts } from '../../types'
import {
  mkSubscriptionApiObj,
  mkOffsetMapApiObj,
  tryMakeEventFromApiObj,
  orderingToApiStr,
} from '../../util'
import * as uri from 'uri-js'

import * as CONSTANTS from '../constants'
import { doLineStreamingRequest } from '../request'

/** @internal */
export const _mkRequestObject = (
  subscriptions: Subscription | Subscription[],
  ordering: Ordering,
  upperBound: OffsetMap,
  lowerBound?: OffsetMap,
): object => {
  const obj: { lowerBound?: object; upperBound: object; subscriptions: object; order: string } = {
    subscriptions: {},
    upperBound: {},
    order: '',
  }

  obj.subscriptions = mkSubscriptionApiObj(subscriptions)
  obj.order = orderingToApiStr(ordering)
  obj.upperBound = mkOffsetMapApiObj(upperBound)

  if (lowerBound && Object.keys(lowerBound).length > 0) {
    obj.lowerBound = mkOffsetMapApiObj(lowerBound)
  } 


  return obj
}

/** @internal */
export const query = (clientOpts: ApiClientOpts) => (opts: QueryOpts) => {
  // Note we add the extra slash here for safety
  const url = clientOpts.Endpoints.EventService.BaseUrl + clientOpts.Endpoints.EventService.Query
  const { host, port, path } = uri.parse(url)

  // Request options
  const requestOptions = {
    hostname: host,
    port: port,
    path: path,
    method: 'POST',
    headers: CONSTANTS.CONTENT_TYPE_JSON_HEADER,
  }

  const body = JSON.stringify(
    _mkRequestObject(opts.subscriptions, opts.ordering, opts.upperBound, opts.lowerBound),
  )

  doLineStreamingRequest({
    requestOptions,
    expectedStatusCode: 200,
    body,
    onLine: line => {
      let obj = {}
      try {
        obj = JSON.parse(line)
      } catch (err) {
        throw `unable to parse line '${line}' as JSON`
      }

      const eitherEvent = tryMakeEventFromApiObj(obj)
      if (typeof eitherEvent === 'string') {
        throw `unable to parse event: ${eitherEvent}`
      }

      opts.onEvent(eitherEvent as Event)
    },
    onDone: opts.onDone,
    onError: opts.onError,
  })
}
