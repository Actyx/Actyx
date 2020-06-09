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
  skipUnlessIntegrationTesting,
  testSemantics,
  testName,
  mkRandId,
} from '../../util/test-util'
import { Client } from '../../client'
import { Event, EventDraft, Subscription } from '../../types'

describe('integration test: publish, then subscribe (small)', () => {
  skipUnlessIntegrationTesting()
  test('publish 4 events and get back via subscription', done => {
    const rand = mkRandId()
    const client = Client()

    client.eventService.publish({
      eventDrafts: [
        EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }),
        EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }),
        EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }),
        EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }),
      ],
      onDone: () => {
        const events: Event[] = []

        const endSubscription = client.eventService.subscribe({
          subscriptions: Subscription.distributed(testSemantics(rand), testName(rand)),
          onEvent: event => {
            events.push(event)
          },
          onDone: () => {
            done()
          },
          onError: error => {
            done(`error subscribing: ${error}`)
          },
        })

        const WAIT_FOR = 2000

        setTimeout(() => {
          expect(events.length).toBe(4)
          endSubscription()
        }, WAIT_FOR)
      },
      onError: error => {
        done(`error publishing: ${error}`)
      },
    })
  })
})
