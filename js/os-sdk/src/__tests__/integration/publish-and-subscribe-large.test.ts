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

describe('integration test: publish, then subscribe (large)', () => {
  skipUnlessIntegrationTesting()

  const NUM_EVENTS = 1000
  const WAIT_FOR = 5000

  test(`publish ${NUM_EVENTS} events and get back via subscription`, done => {
    const rand = mkRandId()
    const client = Client()

    const eventDrafts: EventDraft[] = []
    for (let index = 0; index < NUM_EVENTS; index++) {
      eventDrafts.push(EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }))
    }

    const onFinishedPublishing = () => {
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

      setTimeout(() => {
        expect(events.length).toBe(NUM_EVENTS)
        endSubscription()
      }, WAIT_FOR)
    }

    const pub = () => {
      const ev = eventDrafts.pop()
      if (ev === undefined) {
        onFinishedPublishing()
        return
      }
      client.eventService.publish({
        eventDrafts: ev,
        onDone: pub,
        onError: err => {
          done(`error publishing: ${err}`)
        },
      })
    }
    pub()
  })
})
