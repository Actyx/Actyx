/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-non-null-assertion */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable } from 'rxjs'
import { MockWebSocket } from './multiplexedWebsocket.spec'
import { WebSocketWrapper } from './webSocketWrapper'

let __ws: any
declare const global: any

beforeEach(() => {
  __ws = global.WebSocket
  global.WebSocket = MockWebSocket
})
afterEach(() => {
  global.WebSocket = __ws
  MockWebSocket.clearSockets()
})

describe('webSocketSubject', () => {
  it(
    'should be able to transfer messages when connection is established',
    async () => {
      const rev: string[] = []
      const subject = WebSocketWrapper('ws://socket')
      subject.responses.subscribe(x => rev.push(`${x}`))
      const mockSocket = MockWebSocket.lastSocket!
      subject.sendRequest('"message"')
      // wait some time befor the connection is opened
      await Observable.timer(7000).toPromise()
      mockSocket.open()

      mockSocket.triggerMessage('1')
      mockSocket.triggerMessage('2')

      expect(rev).toEqual(['1', '2'])
      expect(mockSocket.lastMessageSent).toEqual('"message"')
    },
    8000,
  )

  it('work proved by fail', async () => {
    const subject = WebSocketWrapper('ws://socket')
    // subject.subscribe()
    const mockSocket = MockWebSocket.lastSocket!
    // error when socket is not connectiong ( no mockSocket.open() )

    subject.responses.next('"message"')
    expect(mockSocket.lastMessageSent).toEqual(undefined)
  })

  it('should call Hook on connection lost', async () => {
    let hook = false
    WebSocketWrapper('ws://socket', 'ws', () => (hook = true)).responses.subscribe({
      error: _ => ({}),
    })
    const mockSocket = MockWebSocket.lastSocket!
    mockSocket.open()
    mockSocket.triggerClose({
      type: 'close',
      wasClean: false,
    })

    expect(hook).toBeTruthy()
  })

  it('should not call Hook befor connection is established', async () => {
    let hook = false
    WebSocketWrapper('ws://socket', 'ws', () => (hook = true)).responses.subscribe({
      error: _ => ({}),
    })
    const mockSocket = MockWebSocket.lastSocket!
    mockSocket.triggerClose({
      type: 'close',
      wasClean: false,
    })
    mockSocket.open()

    expect(hook).toBeFalsy()
  })
})
