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

import { ApiClientOpts, LogOpts, LogEntryDraft } from '../../types'
import { isLogEntryDraft } from '../../util'
import { doRequest } from '../request'
import { logEntryDraftToApiObj } from './util'
import * as uri from 'uri-js'
import * as CONSTANTS from '../constants'

/** @internal */
export const log = (clientOpts: ApiClientOpts) => (opts: LogOpts | LogEntryDraft) => {
  const url = clientOpts.Endpoints.ConsoleService.BaseUrl + clientOpts.Endpoints.ConsoleService.Logs
  const { host, port, path } = uri.parse(url)

  const iOpts: LogOpts = !isLogEntryDraft(opts) ? opts : { entry: opts }

  doRequest({
    expectedStatusCode: 201,
    requestOptions: {
      hostname: host,
      port: port,
      path: path,
      method: 'POST',
      headers: CONSTANTS.CONTENT_TYPE_JSON_HEADER,
    },
    body: JSON.stringify(logEntryDraftToApiObj(iOpts.entry)),
    onResult: () => {
      if (iOpts.onLogged) {
        iOpts.onLogged()
      }
    },
    onError: iOpts.onError,
  })
}

/** @internal */
export const logPromise = (clientOpts: ApiClientOpts) => (entry: LogEntryDraft) =>
  new Promise<void>((onLogged, onError) => log(clientOpts)({ entry, onLogged, onError }))
