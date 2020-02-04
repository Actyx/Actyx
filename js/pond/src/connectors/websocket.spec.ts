/* eslint-disable @typescript-eslint/no-explicit-any */

import { Server } from 'mock-socket'
import { Observable, ReplaySubject, Subject } from 'rxjs'

import mkWebSocket from './websocket'

const serverEvents: Subject<string> = new Subject()
const serverMsgs: Subject<string> = new Subject()
const url = 'ws://localhost:6666/ws'
let wss: any
let ws: any

beforeEach(() => {
  wss = new Server(url)

  wss.on('connection', (clientWs: any) => {
    ws = clientWs
    ws.on('message', (msg: any) => {
      serverMsgs.next(msg)
      ws.send('pong')
    })
    serverEvents.next('connected')
  })
})

afterEach(() => {
  wss.close()
})

describe('mkWebsocket', () => {
  it('should reconnect given a reconnect strategy', () => {
    const e$ = new Subject()
    const reconnectStrategy: (o: Observable<any>) => Observable<any> = e =>
      e.scan((cnt, err) => {
        if (cnt > 2) {
          e$.next('boom')
          throw err
        }
        e$.next(cnt + 1)
        return cnt + 1
      }, 0)
    const { incoming } = mkWebSocket({ url: 'wrongUrl', reconnectStrategy })

    incoming.subscribe({ error: () => ({}) })

    return expect(
      e$
        .take(4)
        .toArray()
        .toPromise(),
    ).resolves.toEqual([1, 2, 3, 'boom'])
  })

  it('should connect', () => {
    const { incoming } = mkWebSocket({ url })
    incoming.subscribe()
    return expect(serverEvents.take(1).toPromise()).resolves.toEqual('connected')
  })

  it('should send a message and receive messages', () => {
    const resultSelector = (e: MessageEvent) => e.data
    const { incoming, outgoing } = mkWebSocket({ url, resultSelector })
    const output$ = new Subject()
    incoming.subscribe(output$)
    outgoing.next('ping')
    return expect(
      serverMsgs
        .take(1)
        .concat(output$.take(1))
        .toArray()
        .toPromise(),
    ).resolves.toEqual(['ping', 'pong'])
  })

  it('should emit the connection state', () => {
    const output$ = new ReplaySubject(10)
    const { incoming, statusObservable } = mkWebSocket({ url })
    statusObservable.subscribe(output$)
    incoming.subscribe()
    wss.close()
    return expect(
      output$
        .take(2)
        .toArray()
        .toPromise(),
    ).resolves.toEqual(['closed', 'open'])
  })
})
