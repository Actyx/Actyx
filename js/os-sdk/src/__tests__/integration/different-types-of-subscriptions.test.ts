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
import { Event, Subscription, EventDraft } from '../../types'
import { isDeepStrictEqual } from 'util'

describe('integration test: try different types of subscriptions', () => {
  skipUnlessIntegrationTesting()

  test(`publish and the subscribe using only test semantics`, done => {
    const rand = mkRandId()
    const client = Client()
    client.eventService.publish({
      eventDrafts: EventDraft.make(testSemantics(rand), testName(rand), { abc: 123 }),
      onDone: () => {
        const events: Event[] = []
        const endSubscription = client.eventService.subscribe({
          subscriptions: Subscription.wildcard(testSemantics(rand)),
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
          expect(events.length).toBe(1)
          endSubscription()
        }, WAIT_FOR)
      },
      onError: error => {
        done(`error publishing: ${error}`)
      },
    })
  })

  test(`publish and the subscribe using only test name`, done => {
    const rand = mkRandId()
    const client = Client()
    client.eventService.publish({
      eventDrafts: EventDraft.make(testSemantics(rand), testName(rand), { abc: 123 }),
      onDone: () => {
        const events: Event[] = []
        const endSubscription = client.eventService.subscribe({
          subscriptions: { streamName: testName(rand) },
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
          expect(events.length).toBe(1)
          endSubscription()
        }, WAIT_FOR)
      },
      onError: error => {
        done(`error publishing: ${error}`)
      },
    })
  })

  test(`publish, then subscribe with complete wildcard and ensure we got our event`, done => {
    const rand = mkRandId()
    const client = Client()

    const ourPayload = { testId: rand }
    const ourEvent = EventDraft.make(testSemantics(rand), testName(rand), ourPayload)

    client.eventService.publish({
      eventDrafts: ourEvent,
      onDone: () => {
        const events: Event[] = []
        const endSubscription = client.eventService.subscribe({
          subscriptions: Subscription.everything(),
          onEvent: event => {
            if (isDeepStrictEqual(event.payload, ourPayload)) {
              events.push(event)
            }
          },
          onDone: () => {
            done()
          },
          onError: error => {
            done(`error subscribing: ${error}`)
          },
        })
        // FIXME: this probably doesn't work as intended. What happens if this
        // wait time is not long enough to get the most recently posted event?
        const WAIT_FOR = 20000
        setTimeout(() => {
          expect(events.length).toBe(1)
          expect(events[0].payload).toStrictEqual({ testId: rand })
          endSubscription()
        }, WAIT_FOR)
      },
      onError: error => {
        done(`error publishing: ${error}`)
      },
    })
  })
})
