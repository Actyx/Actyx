/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { EventEmitter } from 'events'
import { Subject, fromEvent } from '../../node_modules/rxjs'
import { first } from '../../node_modules/rxjs/operators'
import { decorateEConnRefused } from './errors'
import log from './log'
import { MessageEvent } from 'isomorphic-ws'
import * as WebSocket from 'isomorphic-ws'

export interface WebSocketWrapper<TRequest, TResponse> {
  responses(): Subject<TResponse>

  sendRequest(req: TRequest): void

  close(): void
}

export type WebSocketConstructor = {
  create: (url: string, protocol?: string | string[] | undefined) => WebSocket
}

const DefaultWebsocketConstructor: WebSocketConstructor = {
  create: (url, protocol) => new WebSocket(url, protocol),
}

export const WebSocketWrapper = <TRequest, TResponse>(
  url: string,
  protocol?: string | string[],
  onConnectionLost?: () => void,
  /** Automatic reconnect timer. THIS ONLY WORKS ON V1, because on V2 the token expires. For V2, use `v2/reconnectingWs`. */
  reconnectTimer?: number,
  wsConstructor?: WebSocketConstructor,
): WebSocketWrapperImpl<TRequest, TResponse> => {
  return new WebSocketWrapperImpl<TRequest, TResponse>(
    wsConstructor || DefaultWebsocketConstructor,
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
  binaryType?: 'blob' | 'arraybuffer'
  socketEvents = new EventEmitter()

  responses(): Subject<TResponse> {
    if (this.error) {
      throw new Error(this.error)
    }

    if (!this.tryConnect) {
      throw new Error('This WS has been closed by a call to close()')
    }

    return this.responsesInner
  }

  private responsesInner = new Subject<TResponse>()

  private wasOpened = false
  private tryConnect = true
  private error: string | undefined = undefined

  sendRequest(req: TRequest): void {
    const msg = JSON.stringify(req)

    // send message to the existion socket, or wait till a connection is established to send the message out
    if (!this.socket) {
      log.ws.error('send message to undefined socket')
    } else if (this.socket.readyState === 1) {
      this.socket.send(msg)
    } else {
      log.ws.debug('Delaying request until socket is open:', msg)
      fromEvent<WebSocket>(this.socketEvents, 'connected')
        .pipe(first())
        .subscribe((s) => s.send(msg))
    }
  }

  close(): void {
    this.tryConnect = false
    this.socket && this.socket.close(1000, 'Application shutting down')
  }

  constructor(
    private readonly wsConstrucor: WebSocketConstructor,
    private readonly url: string,
    private readonly protocol: string | string[] | undefined,
    private readonly onConnectionLost: (() => void) | undefined,
    // If unset, disable automatic reconnect
    private readonly reconnectTimer: number | undefined,
  ) {
    log.ws.info('establishing Pond API WS', url)

    this.url = url

    this.connect()
  }

  resultSelector(e: MessageEvent): TResponse {
    const asStr = typeof e.data === 'string' ? e.data : e.data.toString()
    return JSON.parse(asStr) as TResponse
  }

  /**
   * Create the WebSocket and listen to the events
   * The onConnectionLost hook is called on close, when the connetion was already connected
   */
  private createSocket(
    onMessage: (ev: MessageEvent) => any,
    binaryType?: 'arraybuffer' | 'nodebuffer' | 'fragments',
  ): WebSocket {
    const { protocol, url, socketEvents } = this

    const socket = (this.socket = protocol
      ? this.wsConstrucor.create(url, protocol)
      : this.wsConstrucor.create(url))
    if (binaryType) {
      socket.binaryType = binaryType
    }
    socket.onerror = (err) => {
      const originalMsg = (err as any).message

      const msg = decorateEConnRefused(originalMsg, url)

      this.error = msg

      try {
        log.ws.error(msg)
        this.responsesInner.error(msg)
      } catch (err) {
        const errMsg = `Error while passing websocket error message ${msg} up the chain!! -- ${err}`
        console.error(errMsg)
        log.ws.error(errMsg)
      }
    }
    socket.onmessage = onMessage
    socket.onclose = (err) => {
      if (!this.tryConnect || !this.wasOpened) {
        // Orderly close desired by the user, or we did not even manage to connect
        return
      }

      if (this.onConnectionLost) {
        this.onConnectionLost()
      }

      if (this.reconnectTimer) {
        this.socket = undefined
        this.responsesInner = new Subject()

        const handle = setInterval(() => {
          this.connect() && clearInterval(handle)
        }, this.reconnectTimer)
      } else {
        this.error = 'WS connection errored and closed for good'
      }

      this.responsesInner.error(`Connection lost with reason '${err.reason}', code ${err.code}`)
    }

    socket.onopen = () => {
      this.wasOpened = true
      log.ws.debug('WS open to', url)
      socketEvents.emit('connected', socket)
    }

    return socket
  }

  private connect(): boolean {
    const observer = this.responsesInner
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
      return true
    } catch (e) {
      log.ws.error('WebSocket not supported on this plattform')
      observer.error(e)
      return false
    }
  }
}
