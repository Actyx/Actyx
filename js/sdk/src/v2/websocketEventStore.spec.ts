/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { EventStoreV2 } from '.'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { MockWebSocket } from './multiplexedWebsocket.spec'
import { getNodeId, WebsocketEventStore } from './websocketEventStore'

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
  const createEventStore = async (): Promise<EventStoreV2> => {
    const multiplexer = new MultiplexedWebsocket({ url: 'ws://mock' })
    const sourceId = await getNodeId(multiplexer)
    return new WebsocketEventStore(multiplexer, sourceId)
  }
  it('request sourceId on create', async () => {
    const store = await createEventStore()
    return expect(store.nodeId).toEqual('MOCK')
  })
})
