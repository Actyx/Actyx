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
import { ApiClientOpts } from '../types'
import { getAxEventServiceUri } from '../util'
import { DEFAULT_EVENT_SERVICE_URI } from './constants'

/**
 * These are the default SDK client options. They should be fine for almost
 * all use cases.
 */
export const DefaultClientOpts: ApiClientOpts = {
  Endpoints: {
    EventService: {
      BaseUrl: getAxEventServiceUri(DEFAULT_EVENT_SERVICE_URI),
      Subscribe: 'v1/events/subscribe',
      Offsets: 'v1/events/offsets',
      Query: 'v1/events/query',
      Publish: 'v1/events/publish',
    },
  }
}

/** @internal */
export const CONTENT_TYPE_JSON_HEADER = { 'Content-Type': 'application/json' }

/** @internal */
export const ACCEPT_NDJSON_HEADER = { Accept: 'application/x-ndjson' }
