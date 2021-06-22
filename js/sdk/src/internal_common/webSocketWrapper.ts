/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { EventEmitter } from 'events'
import { Observable, ReplaySubject, Subject } from 'rxjs'
import { isNode } from '../util'
import { root } from '../util/root'
import log from './log'

if (isNode) {
  root.WebSocket = require('ws')
}

export interface WebSocketWrapper<TRequest, TResponse> {
  readonly responses: Subject<TResponse>

  sendRequest(req: TRequest): void

  close(): void
}

export const WebSocketWrapper = <TRequest, TResponse>(
  url: string,
  protocol?: string | string[],
  onConnectionLost?: () => void,
  reconnectTimer: number = 1000,
): WebSocketWrapperImpl<TRequest, TResponse> => {
  return new WebSocketWrapperImpl<TRequest, TResponse>(
    url,
    protocol,
    onConnectionLost,
    reconnectTimer,
  )
}

/**
 * Copied from `rxjs/observable/dom/WebSocketSubject.ts`
 * But the WebSocket and the Subscription is not connected any more.
 * So the Socket can connect later and the Subscriptions are still valid
 */
class WebSocketWrapperImpl<TRequest, TResponse> implements WebSocketWrapper<TRequest, TResponse> {
  socket?: WebSocket
  WebSocketCtor: { new (url: string, protocol?: string | string[]): WebSocket }
  binaryType?: 'blob' | 'arraybuffer'
  socketEvents = new EventEmitter()

  readonly responses: Subject<TResponse>

  private connected = false

  private requests = new ReplaySubject()

  sendRequest(req: TRequest): void {
    this.requests.next(req)
  }

  close(): void {
    this.requests.complete()
  }

  constructor(
    private readonly url: string,
    private readonly protocol?: string | string[],
    private readonly onConnectionLost?: () => void,
    private readonly reconnectTimer: number = 1000,
  ) {
    if (!root.WebSocket) {
      log.ws.error('WebSocket not supported on this plattform')
      throw new Error('no WebSocket constructor can be found')
    }
    this.WebSocketCtor = root.WebSocket
    this.responses = new Subject<TResponse>()
    this.url = url

    this.connect()
  }

  resultSelector(e: MessageEvent): TResponse {
    return JSON.parse(e.data) as TResponse
  }

  static create<TRequest, TResponse>(
    url: string,
    protocol?: string | string[],
    onConnectionLost?: () => void,
    reconnectTimer: number = 1000,
  ): WebSocketWrapperImpl<TRequest, TResponse> {
    return new WebSocketWrapperImpl<TRequest, TResponse>(
      url,
      protocol,
      onConnectionLost,
      reconnectTimer,
    )
  }

  private resetState(): void {
    const socket = this.socket
    this.socket = undefined
    if (socket && socket.readyState === 1) {
      socket.close()
    }

    this.requests = new ReplaySubject()
  }

  /**
   * Create the WebSocket and listen to the events
   * The onConnectionLost hook is called on close, when the connetion was already connected
   */
  private createSocket(
    onMessage: (this: WebSocket, ev: MessageEvent) => any,
    binaryType?: 'blob' | 'arraybuffer',
  ): WebSocket {
    const { WebSocketCtor, protocol, url, socketEvents } = this

    const socket = (this.socket = protocol
      ? new WebSocketCtor(url, protocol)
      : new WebSocketCtor(url))
    if (binaryType) {
      socket.binaryType = binaryType
    }
    socket.onerror = err => {
      const originalMsg = (err as any).message

      const msg = originalMsg.includes('ECONNREFUSED')
        ? 'Error: unable to connect to Actyx. Is it running? -- Error: ' + originalMsg
        : 'Error in connection to Actyx service: ' + originalMsg

      try {
        log.ws.error(msg)
        this.responses && this.responses.error(msg)
      } catch (err) {
        const errMsg = `Error while passing websocket error message ${msg} up the chain!! -- ${err}`
        console.error(errMsg)
        log.ws.error(errMsg)
      }
    }
    socket.onmessage = onMessage
    socket.onclose = err => {
      // Can be removed, when the hot reconnect is possible
      if (this.connected) {
        if (this.onConnectionLost) {
          this.onConnectionLost()
        }
        this.responses &&
          this.responses.error(`Connection lost with reason '${err.reason}', code ${err.code}`)
      } else {
        Observable.timer(this.reconnectTimer).subscribe(() =>
          this.createSocket(onMessage, binaryType),
        )
      }
    }

    socket.onopen = () => {
      this.connected = true
      socketEvents.emit('connected', socket)
    }

    return socket
  }

  private connect(): void {
    const observer = this.responses
    try {
      const onmessage = (e: MessageEvent) => {
        try {
          const result = this.resultSelector(e)
          observer.next(result)
        } catch (e) {
          observer.error(e)
        }
      }
      this.socket = this.createSocket(onmessage)
    } catch (e) {
      log.ws.error('WebSocket not supported on this plattform')
      observer.error(e)
      return
    }

    this.requests.subscribe(
      msg => {
        // send message to the existion socket, or wait till a connection is established to send the message out
        if (!this.socket) {
          log.ws.error('send message to undefined socket')
        } else if (this.socket.readyState === 1) {
          this.socket.send(JSON.stringify(msg))
        } else {
          Observable.fromEvent<WebSocket>(this.socketEvents, 'connected')
            .first()
            .subscribe(s => s.send(JSON.stringify(msg)))
        }
      },
      (err: any) => {
        if (err && err.code) {
          this.socket && this.socket.close(err.code, err.reason)
        } else {
          observer.error(
            new TypeError(
              'WebSocketSubject.error must be called with an object with an error code, ' +
                'and an optional reason: { code: number, reason: string }',
            ),
          )
        }
        this.resetState()
      },
      () => {
        this.resetState()
      },
    )
  }
}
