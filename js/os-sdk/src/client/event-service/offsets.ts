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
import { OffsetMap, ApiClientOpts, OffsetsOpts } from '../../types'
import { tryMakeOffsetMapFromApiObj } from '../../util'
import * as uri from 'uri-js'

import { doRequest } from '../request'

/** @internal */
export const offsets = (clientOpts: ApiClientOpts) => (opts: OffsetsOpts) => {

  // Note we add the extra slash here for safety
  const url = clientOpts.Endpoints.EventService.BaseUrl + clientOpts.Endpoints.EventService.Offsets
  const { host, port, path } = uri.parse(url)

  // Request options
  const requestOptions = {
    hostname: host,
    port: port,
    path: path,
    method: 'GET',
  }
  doRequest({
    requestOptions,
    expectedStatusCode: 200,
    onResult: res => {
      let obj = {}
      try {
        obj = JSON.parse(res)
      } catch (err) {
        if (opts.onError) {
          opts.onError(err)
        }
        return
      }

      const eitherOffsetMap = tryMakeOffsetMapFromApiObj(obj)
      if (typeof eitherOffsetMap === 'string') {
        if (opts.onError) {
          opts.onError(eitherOffsetMap)
        }
        return
      }
      opts.onOffsets(eitherOffsetMap as OffsetMap)
    },
    onError: opts.onError,
  })
}
