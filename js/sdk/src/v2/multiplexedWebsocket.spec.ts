/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
/* eslint-disable @typescript-eslint/no-non-null-assertion */

import {
  fromNullable,
  map as mapO,
  filter as filterO,
  getOrElse as getOrElseO,
  none,
} from 'fp-ts/lib/Option'
import { pipe } from 'fp-ts/lib/function'
import { range, takeWhile } from 'ramda'
import { timer, Subject } from '../../node_modules/rxjs'
import { MultiplexedWebsocket, ResponseMessage } from './multiplexedWebsocket'
import { RequestTypes } from './websocketEventStore'
import { lastValueFrom } from '../../node_modules/rxjs'
import { tap, first, toArray } from '../../node_modules/rxjs/operators'
import {
  RequestMessage,
  RequestMessageType,
  ResponseMessageType,
} from '../internal_common/multiplexedWebSocket'

afterEach(() => {
  MockWebSocket.clearSockets()
})
// poor man's generator
const msgGen: () => ((requestId: number) => ResponseMessage[])[] = () => {
  const numberOfTests = Math.max(2, Math.random() * 30)
  const numberOfMessages = Math.random() * 500
  const msgType = (requestId: number): ResponseMessage => {
    const rnd = Math.random()
    if (rnd < 0.75) {
      const a: ResponseMessage = {
        type: ResponseMessageType.Next,
        requestId,
        payload: [[Math.floor(Math.random() * 100)]],
      }
      return a
    } else if (rnd < 0.9) {
      const a: ResponseMessage = {
        type: ResponseMessageType.Error,
        requestId,
        kind: 'Some random error...',
      }
      return a
    } else {
      const a: ResponseMessage = { type: ResponseMessageType.Complete, requestId }
      return a
    }
  }
  return range(0, numberOfTests).map(
    () => (requestId: number) => range(0, numberOfMessages).map(() => msgType(requestId)),
  )
}

describe('multiplexedWebsocket', () => {
  it('should report connection errors', async () => {
    const openObserver = new Subject()
    let opened = 0
    openObserver.subscribe({ next: () => (opened += 1) })

    const closeObserver = new Subject()
    let closed = 0
    closeObserver.subscribe({ next: () => (closed += 1) })

    const s = new MultiplexedWebsocket(
      {
        url: 'ws://socket',
        openObserver,
        closeObserver,
        WebSocketCtor: MockWebSocketConstructor,
        maxConcurrentRequests: none,
      },
      100,
    )

    const subject = s.errors()

    const pErr1 = lastValueFrom(subject.pipe(first()))
    MockWebSocket.lastSocket!.trigger('error', { message: 'destination unreachable' })
    expect(await pErr1).toEqual({ message: 'destination unreachable' })

    // await redial
    await new Promise((res) => setTimeout(res, 500))
    expect(MockWebSocket.sockets).toHaveLength(2)

    expect([opened, closed]).toEqual([0, 0])
    const ws1 = MockWebSocket.lastSocket!
    ws1.open()
    expect([opened, closed]).toEqual([1, 0])

    const pErr2 = lastValueFrom(subject.pipe(first()))
    const pReq1 = lastValueFrom(s.request(RequestTypes.Offsets).pipe(first())).catch((err) =>
      expect(err).toEqual(new Error('{"message":"broken"}')),
    )

    expect(ws1.lastMessageSent).toEqual({
      type: 'request',
      serviceId: 'offsets',
      requestId: 0,
      payload: null,
    })
    ws1.trigger('error', { message: 'broken' })

    expect(await pErr2).toEqual({ message: 'broken' })
    expect(await pReq1).toBeUndefined()
  })

  it('should just work', async () => {
    const testArr = msgGen()
    const multiplexer = new MultiplexedWebsocket({
      url: 'ws://socket',
      WebSocketCtor: MockWebSocketConstructor,
      maxConcurrentRequests: none,
    })
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const socket = MockWebSocket.lastSocket!
    socket.open()

    // Make sure no assertions are missed. Because async and silent success..
    expect.assertions(testArr.length * 4)
    for (const msgFn of testArr) {
      const requestType = 'someRequest'
      const receivedVals: string[] = []
      const res = lastValueFrom(
        multiplexer.request(requestType, { from: 1, to: 4 }).pipe(
          tap((x) => receivedVals.push(x as string)),
          toArray(),
        ),
      )

      // For testing with real webSocket, please remove assertions against socket.lastMessageSent
      // as this is a particular of the MockWebSocket implementation.
      // You probably want only to check the contents of the res or receivedVals...
      const { requestId } = socket.lastMessageSent
      // Assert initial request message has been sent
      const initialRequest: RequestMessage = {
        type: RequestMessageType.Request,
        requestId,
        serviceId: requestType,
        payload: { from: 1, to: 4 },
      }
      expect(socket.lastMessageSent).toMatchObject(initialRequest)
      const msgs = msgFn(requestId)
      // Sent generated `ResponseMessage`s over the _wire_
      msgs.forEach((response) => socket.triggerMessage(JSON.stringify(response)))
      // Trigger complete in any case
      socket.triggerMessage(JSON.stringify({ type: ResponseMessageType.Complete, requestId }))
      const nextMsgs = takeWhile((x) => x.type === ResponseMessageType.Next, msgs).flatMap(
        (x) => x.type === ResponseMessageType.Next && x.payload,
      )
      const errorIdx = msgs.findIndex((x) => x.type === ResponseMessageType.Error)
      const completeIdx = msgs.findIndex((x) => x.type === ResponseMessageType.Complete)
      const err = (completeIdx === -1 || completeIdx > errorIdx) && msgs[errorIdx]
      if (err) {
        await expect(res).rejects.toEqual(
          // Please, compiler, pretty please..
          new Error((err.type === ResponseMessageType.Error && JSON.stringify(err.kind)) || ''),
        )
        // Some messages were still received
        expect(receivedVals).toEqual(nextMsgs)
        // we always cancel upstream
        expect(socket.lastMessageSent).toMatchObject({
          type: 'cancel',
          requestId,
        })
      } else {
        await expect(res).resolves.toEqual(nextMsgs)
        expect(socket.lastMessageSent).toMatchObject({
          type: 'cancel',
          requestId,
        })
        expect(true).toBeTruthy() // Keep number of assertions symmetrical for all paths
      }
    }
  })
})

class MessageEvent {
  type: string = 'message'
  origin: string = 'mockorigin'
  data: string
  source: any

  constructor(data: any, target: any) {
    this.data = JSON.stringify(data)

    this.source = target
  }
}

// courtesy of https://github.com/ReactiveX/rxjs/blob/master/spec/observables/dom/webSocket-spec.ts

type MockWebSocketMsgHandlerResult = { name: string; res: MessageEvent[] }
export type MockWebSocketMsgHandler = (
  data: RequestMessage,
) => MockWebSocketMsgHandlerResult | undefined
export type MockWebSocketAutoRespons = {
  socket: MockWebSocket
  messageHandler?: MockWebSocketMsgHandler
}

export class MockWebSocket {
  static autoResponse = false
  static sockets: MockWebSocketAutoRespons[] = []
  static get lastSocket(): MockWebSocket | undefined {
    const [socket] = MockWebSocket.sockets
    return socket ? socket.socket : undefined
  }

  static clearSockets(): void {
    MockWebSocket.sockets.length = 0
    MockWebSocket.messageHandler = undefined
  }
  static messageHandler?: MockWebSocketMsgHandler

  static mkResponse(content: ResponseMessage): MessageEvent {
    return new MessageEvent(content, global)
  }

  autoResponse: boolean = false
  sent: any[] = []
  readyState: number = 0
  closeCode: any
  closeReason: any
  binaryType?: string
  socketMessageHandler?: MockWebSocketMsgHandler

  constructor(public url: string, public protocol?: string | string[] | undefined) {
    MockWebSocket.sockets.unshift({ socket: this, messageHandler: MockWebSocket.messageHandler })
    this.socketMessageHandler = MockWebSocket.messageHandler
    this.autoResponse = MockWebSocket.autoResponse
    if (this.autoResponse) {
      timer(50).subscribe((_) => this.open())
    }
  }

  send(data: any): void {
    this.sent.push(data)
    if (!this.autoResponse) {
      return
    }
    const actions = pipe(
      fromNullable(this.socketMessageHandler),
      mapO((f) => f(JSON.parse(data))),
      filterO(
        (res: MockWebSocketMsgHandlerResult | undefined): res is MockWebSocketMsgHandlerResult =>
          res !== undefined,
      ),
      getOrElseO(() => {
        const request: RequestMessage = JSON.parse(data)

        if (
          request.type === RequestMessageType.Request &&
          request.serviceId === RequestTypes.Offsets
        ) {
          return {
            name: 'message',
            res: [
              MockWebSocket.mkResponse({
                type: ResponseMessageType.Next,
                requestId: request.requestId,
                payload: [{ offsets: {} }],
              }),
              MockWebSocket.mkResponse({
                type: ResponseMessageType.Complete,
                requestId: request.requestId,
              }),
            ],
          }
        }
        return { name: '', res: [] }
      }),
    )

    actions && actions.res.forEach((ev) => this.trigger(actions.name, ev))
  }

  get lastMessageSent(): any | undefined {
    const sent = this.sent
    const length = sent.length

    return length > 0 ? JSON.parse(sent[length - 1]) : undefined
  }

  triggerClose(e: any): void {
    this.readyState = 3
    this.trigger('close', e)
  }

  triggerMessage(data: any): void {
    const messageEvent = {
      data,
      origin: 'mockorigin',
      ports: undefined as any,
      source: global,
    }

    this.trigger('message', messageEvent)
  }

  open(): void {
    this.readyState = 1
    this.trigger('open', {})
  }

  close(code: any, reason: any): void {
    if (this.readyState < 2) {
      this.readyState = 2
      this.closeCode = code
      this.closeReason = reason
      this.triggerClose({ wasClean: true })
    }
  }

  trigger(name: string, e: any) {
    const call = (this as any)['on' + name]
    if (typeof call === 'function') {
      call(e)
    }
  }
}

export const MockWebSocketConstructor: new (
  url: string,
  protocols?: string | string[] | undefined,
) => globalThis.WebSocket = <any>MockWebSocket
