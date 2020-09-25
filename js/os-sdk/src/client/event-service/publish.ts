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
import { EventDraft, ApiClientOpts, PublishOpts, PublishPromiseOpts } from '../../types'
import * as uri from 'uri-js'

import * as CONSTANTS from '../constants'
import { doRequest } from '../request'

/** @internal */
export const _mkRequestObject = (eventDrafts: EventDraft[]): object => {
  const obj: { data: object[] } = { data: [] }
  eventDrafts.map((ed: EventDraft) => {
    obj.data.push({
      name: ed.streamName,
      payload: ed.payload,
      semantics: ed.streamSemantics,
    })
  })
  return obj
}

/** @internal */
export const publish = (clientOpts: ApiClientOpts) => (opts: PublishOpts) => {
  const url = clientOpts.Endpoints.EventService.BaseUrl + clientOpts.Endpoints.EventService.Publish
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
    _mkRequestObject(Array.isArray(opts.eventDrafts) ? opts.eventDrafts : [opts.eventDrafts]),
  )

  //doRequest(options, body, 201, onDone, onError)
  doRequest({
    requestOptions,
    expectedStatusCode: 201,
    body: body,
    onResult: () => {
      if (opts.onDone) {
        opts.onDone()
      }
    },
    onError: opts.onError,
  })
}

/** @internal */
export const publishPromise = (clientOpts: ApiClientOpts) => (opts: PublishPromiseOpts) =>
  new Promise<void>((res, rej) => publish(clientOpts)({ ...opts, onDone: res, onError: rej }))
