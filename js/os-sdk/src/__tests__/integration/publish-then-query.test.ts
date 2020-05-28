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
import { Event, EventDraft, Ordering, Subscription } from '../../types'

describe('integration test: publish, then query', () => {
  skipUnlessIntegrationTesting()

  const NUM_EVENTS = 100

  test(`publish ${NUM_EVENTS} events and then check if query works`, done => {
    const rand = mkRandId()
    const client = Client()

    const eventDrafts: EventDraft[] = []
    for (let index = 0; index < NUM_EVENTS; index++) {
      eventDrafts.push(EventDraft.make(testSemantics(rand), testName(rand), { foo: 'bar' }))
    }

    const onFinishedPublishing = () => {
      // Get offsets
      client.eventService.offsets({
        onOffsets: offsets => {
          const toOffsets = { ...offsets }
          const fromOffsets = { ...toOffsets }
          Object.keys(fromOffsets).forEach(sourceId => {
            fromOffsets[sourceId] = -1
          })

          const events: Event[] = []

          client.eventService.query({
            upperBound: toOffsets,
            ordering: Ordering.Lamport,
            subscriptions: Subscription.distributed(testSemantics(rand), testName(rand)),
            onEvent: event => {
              events.push(event)
            },
            onDone: () => {
              expect(events.length).toBe(NUM_EVENTS)
              done()
            },
            onError: error => {
              done(`error querying: ${error}`)
            },
          })
        },
        onError: error => {
          done(`error getting offsets: ${error}`)
        },
      })
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
