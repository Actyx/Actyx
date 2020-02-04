/* eslint-disable @typescript-eslint/no-explicit-any */

import { EventStore } from '.'
import { FishName, Semantics, Timestamp } from '../types'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { MockWebSocket } from './multiplexedWebsocket.spec'
import { UnstoredEvents } from './types'
import { getSourceId, WebsocketEventStore } from './websocketEventStore'

let __ws: any
declare const global: any

beforeEach(() => {
  __ws = global.WebSocket
  MockWebSocket.autoResponse = true
  global.WebSocket = MockWebSocket
})
afterEach(() => {
  global.WebSocket = __ws
  MockWebSocket.autoResponse = false
  MockWebSocket.clearSockets()
})

describe('websocketEventStore', () => {
  const jellyFishEvent = () => ({
    name: FishName.of('testFish'),
    payload: { type: 'empty' },
    semantics: Semantics.jelly('ax.Test'),
    timestamp: Timestamp.now(),
  })
  const createEventStore = async (): Promise<EventStore> => {
    const multiplexer = new MultiplexedWebsocket({ url: 'ws://mock' })
    const sourceId = await getSourceId(multiplexer)
    return new WebsocketEventStore(multiplexer, sourceId)
  }
  it('request sourceId on create', async () => {
    const store = await createEventStore()
    return expect(store.sourceId).toEqual('MOCK')
  })

  it('Jelly events should just be returned, and not sent over the wire', async () => {
    const store = await createEventStore()

    const jellyEvent = jellyFishEvent()
    const jellyFishEvents: UnstoredEvents = [jellyEvent]

    await expect(store.persistEvents(jellyFishEvents).toPromise()).resolves.toEqual([
      { ...jellyEvent, psn: 0, lamport: 0, sourceId: store.sourceId },
    ])
    // getting the source id and cancel it, nothing more!
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect(MockWebSocket.lastSocket!.sent.length).toEqual(2)
  })
})
