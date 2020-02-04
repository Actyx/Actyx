/* eslint-disable @typescript-eslint/no-explicit-any */

import { fromNullable } from 'fp-ts/lib/Option'
import { range, takeWhile } from 'ramda'
import { Observable } from 'rxjs'
import { SourceId } from '../types'
import {
  MultiplexedWebsocket,
  Request,
  ResponseMessage,
  ResponseMessageType,
} from './multiplexedWebsocket'
import { RequestTypes } from './websocketEventStore'

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
        payload: Math.floor(Math.random() * 100),
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
  return range(0, numberOfTests).map(() => (requestId: number) =>
    range(0, numberOfMessages).map(() => msgType(requestId)),
  )
}

describe('multiplexedWebsocket', () => {
  it('should just work', async () => {
    const testArr = msgGen()
    const store = new MultiplexedWebsocket({ url: 'ws://socket' })
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const socket = MockWebSocket.lastSocket!
    socket.open()

    // Make sure no assertions are missed. Because async and silent success..
    expect.assertions(testArr.length * 4)
    for (const msgFn of testArr) {
      const requestType = 'someRequest'
      const receivedVals: string[] = []
      const res = store
        .request(requestType, { from: 1, to: 4 })
        .do(x => receivedVals.push(x as string))
        .toArray()
        .toPromise()

      // For testing with real webSocket, please remove assertions against socket.lastMessageSent
      // as this is a particular of the MockWebSocket implementation.
      // You probably want only to check the contents of the res or receivedVals...
      const { requestId } = socket.lastMessageSent
      // Assert initial request message has been sent
      const initialRequest: Request = {
        type: 'request',
        requestId,
        serviceId: requestType,
        payload: { from: 1, to: 4 },
      }
      expect(socket.lastMessageSent).toMatchObject(initialRequest)
      const msgs = msgFn(requestId)
      // Sent generated `ResponseMessage`s over the _wire_
      msgs.forEach(response => socket.triggerMessage(JSON.stringify(response)))
      // Trigger complete in any case
      socket.triggerMessage(JSON.stringify({ type: ResponseMessageType.Complete, requestId }))
      const nextMsgs = takeWhile(x => x.type === ResponseMessageType.Next, msgs).map(
        x => x.type === ResponseMessageType.Next && x.payload,
      )
      const errorIdx = msgs.findIndex(x => x.type === ResponseMessageType.Error)
      const completeIdx = msgs.findIndex(x => x.type === ResponseMessageType.Complete)
      const err = (completeIdx === -1 || completeIdx > errorIdx) && msgs[errorIdx]
      if (err) {
        await expect(res).rejects.toEqual(
          // Please, compiler, pretty please..
          new Error((err.type === ResponseMessageType.Error && JSON.stringify(err.kind)) || ''),
        )
        // Some messages were still received
        expect(nextMsgs).toEqual(nextMsgs)
        // don't cancel upstream, as upstream errored
        expect(socket.lastMessageSent).toMatchObject({
          type: 'request',
          serviceId: requestType,
          requestId,
        })
      } else {
        await expect(res).resolves.toEqual(nextMsgs)
        // don't cancel upstream, as upstream Completed
        expect(socket.lastMessageSent).toMatchObject({
          type: 'request',
          serviceId: requestType,
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

export type MockWebSocketMsgHandler = (
  data: Request,
) => { name: string; res: MessageEvent[] } | undefined
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
  handlers: any = {}
  readyState: number = 0
  closeCode: any
  closeReason: any
  binaryType?: string
  socketMessageHandler?: MockWebSocketMsgHandler

  constructor(public url: string, public protocol?: string | string[] | undefined) {
    MockWebSocket.sockets.push({ socket: this, messageHandler: MockWebSocket.messageHandler })
    this.socketMessageHandler = MockWebSocket.messageHandler
    this.autoResponse = MockWebSocket.autoResponse
    if (this.autoResponse) {
      Observable.timer(50).subscribe(_ => this.open())
    }
  }

  send(data: any): void {
    this.sent.push(data)
    if (!this.autoResponse) {
      return
    }
    const actions = fromNullable(this.socketMessageHandler)
      .map(f => f(JSON.parse(data)))
      .filter(res => res !== undefined)
      .getOrElseL(() => {
        const request = JSON.parse(data)
        if (request.serviceId === RequestTypes.SourceId) {
          return {
            name: 'message',
            res: [
              MockWebSocket.mkResponse({
                type: ResponseMessageType.Next,
                requestId: request.requestId,
                payload: SourceId.of('MOCK'),
              }),
              MockWebSocket.mkResponse({
                type: ResponseMessageType.Complete,
                requestId: request.requestId,
              }),
            ],
          }
        }
        return { name: '', res: [] }
      })

    actions && actions.res.forEach(ev => this.trigger(actions.name, ev))
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

    const lookup = this.handlers[name]
    if (lookup) {
      for (let i = 0; i < lookup.length; i++) {
        lookup[i](e)
      }
    }
  }
}
