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
/* tslint:disable */
import { Client, EventDraft, Subscription } from './'

// Create client with default options
const client = Client()

const doPublish = () => {
  client.eventService.publish({
    eventDrafts: EventDraft.make('testSemantics', 'testName', { foo: 'bar' }),
    onDone: () => {
      console.log(`Published`)
    },
    onError: error => {
      console.error(`error publishing: ${error}`)
    },
  })
}

console.log('getting offsets...')
client.eventService.offsets({
  onOffsets: offsets => {
    console.log('offsets:')
    console.log(JSON.stringify(offsets, null, 2))

    console.log('starting periodic publishing')
    setInterval(doPublish, 2000)

    console.log('subscribing')
    const stopSubscription = client.eventService.subscribe({
      lowerBound: offsets,
      subscriptions: Subscription.everything(),
      onEvent: event => {
        console.log('Event:')
        console.log(JSON.stringify(event, null, 2))
      },
      onDone: () => {
        console.log(`Subscription done!`)
      },
      onError: error => {
        console.error(`error during subscription: ${error}`)
      },
    })

    const TIMEOUT = 30
    console.log(`Stopping subscription in ${TIMEOUT} seconds`)
    setTimeout(() => {
      stopSubscription()
    }, TIMEOUT * 1000)
  },
  onError: error => {
    console.error(`error getting offsets: ${error}`)
  },
})