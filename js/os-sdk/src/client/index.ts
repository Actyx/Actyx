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
import { ApiClient, EventServiceClient, ApiClientOpts } from '../types'
import { subscribe } from './event-service/subscribe'
import { query } from './event-service/query'
import { offsets } from './event-service/offsets'
import { publish } from './event-service/publish'
import { DefaultClientOpts } from './default-opts'

/** @internal */
const addTrailingSlashToBaseUrlsIfNecessary = (opts: ApiClientOpts): ApiClientOpts => {
  if (opts.Endpoints.EventService.BaseUrl.endsWith('/')) {
    return opts;
  }
  return {
    ...opts,
    Endpoints: {
      ...opts.Endpoints,
      EventService: {
        ...opts.Endpoints.EventService,
        BaseUrl: opts.Endpoints.EventService.BaseUrl + '/'
      }
    }
  }
}

/** @internal */
const eventServiceClient = (opts: ApiClientOpts): EventServiceClient => ({
  subscribe: subscribe(addTrailingSlashToBaseUrlsIfNecessary(opts)),
  query: query(addTrailingSlashToBaseUrlsIfNecessary(opts)),
  publish: publish(addTrailingSlashToBaseUrlsIfNecessary(opts)),
  offsets: offsets(addTrailingSlashToBaseUrlsIfNecessary(opts)),
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
export const Client = (opts: ApiClientOpts = DefaultClientOpts): ApiClient => {
  return {
    eventService: eventServiceClient(opts),
  }
}
