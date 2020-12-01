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
import { ApiClient, EventServiceClient, ApiClientOpts, ConsoleServiceClient } from '../types'
import { subscribe, subscribeStream } from './event-service/subscribe'
import { query, queryStream } from './event-service/query'
import { offsets, offsetsPromise } from './event-service/offsets'
import { publish, publishPromise } from './event-service/publish'
import { DefaultClientOpts } from './default-opts'
import { log, logPromise } from './console-service/log'
import { createSimpleLogger } from './console-service/simple-logger'

/** @internal */
const addTrailingSlashToBaseUrlsIfNecessary = (opts: ApiClientOpts): ApiClientOpts => {
  const opts2: ApiClientOpts = JSON.parse(JSON.stringify(opts))
  const events = opts2.Endpoints.EventService
  if (!events.BaseUrl.endsWith('/')) {
    events.BaseUrl = events.BaseUrl + '/'
  }
  const console = opts2.Endpoints.ConsoleService
  if (!console.BaseUrl.endsWith('/')) {
    console.BaseUrl = console.BaseUrl + '/'
  }
  return opts2
}

/** @internal */
const eventServiceClient = (opts: ApiClientOpts): EventServiceClient => ({
  subscribe: subscribe(opts),
  subscribeStream: subscribeStream(opts),
  query: query(opts),
  queryStream: queryStream(opts),
  publish: publish(opts),
  publishPromise: publishPromise(opts),
  offsets: offsets(opts),
  offsetsPromise: offsetsPromise(opts),
})

/** @internal */
const consoleServiceClient = (opts: ApiClientOpts): ConsoleServiceClient => ({
  log: log(opts),
  logPromise: logPromise(opts),
  SimpleLogger: createSimpleLogger(opts),
})

/**
 * This function allows you to create a client. In almost all cases you do not
 * need to provide options since the defaults will work.
 *
 * **Example**
 *
 * ```typescript
 * import { Client } from '@actyx/os-sdk'
 *
 * const ActyxOS = Client()
 * ```
 *
 * In the rare case that you might in fact have to **override clients options**
 * you could do so as follows:
 *
 * ```typescript
 * import { DefaultClientOpts } from '@actyx/os-sdk'
 *
 * const customClient = Client({
 *   ...DefaultClientOpts,
 *   Ports: {
 *     EventService: 5555
 *   }
 * })
 * ```
 */
export const Client = (opts: ApiClientOpts = DefaultClientOpts()): ApiClient => {
  const opts2 = addTrailingSlashToBaseUrlsIfNecessary(opts)
  return {
    eventService: eventServiceClient(opts2),
    consoleService: consoleServiceClient(opts2),
  }
}
