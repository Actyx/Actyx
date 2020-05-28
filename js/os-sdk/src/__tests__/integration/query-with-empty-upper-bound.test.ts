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
import { skipUnlessIntegrationTesting } from '../../util/test-util'
import { Client } from '../../client'
import { Ordering, Subscription } from '../../types'

describe('integration test: query with empty upper bound', () => {
  skipUnlessIntegrationTesting()

  test(`query with an empty upper bound`, done => {
    const client = Client()

    client.eventService.query({
      subscriptions: Subscription.everything(),
      ordering: Ordering.Lamport,
      upperBound: {},
      onEvent: () => {
        done(`unexpectedly got an event`)
      },
      onDone: () => {
        done()
      },
      onError: error => {
        done(`error querying: ${error}`)
      },
    })
  })
})
