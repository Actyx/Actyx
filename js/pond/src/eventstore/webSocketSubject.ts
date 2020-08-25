/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { EventEmitter } from 'events'
import { Observable } from 'rxjs'
import { ReplaySubject } from 'rxjs/ReplaySubject'
import { AnonymousSubject, Subject } from 'rxjs/Subject'
import { Subscriber } from 'rxjs/Subscriber'
import { Subscription } from 'rxjs/Subscription'
import { errorObject } from 'rxjs/util/errorObject'
import { root } from 'rxjs/util/root'
import { tryCatch } from 'rxjs/util/tryCatch'
import log from '../loggers'
import { isNode } from '../util'

if (isNode) {
  const globalAny: any = global

  globalAny.WebSocket = require('ws')
}

/**
 * Copied from `rxjs/observable/dom/WebSocketSubject.ts`
 * But the WebSocket and the Subscription is not connected any more.
 * So the Socket can connect later and the Subscriptions are still valid
 */
export class WebSocketSubject<T> extends AnonymousSubject<T> {
  socket?: WebSocket
  WebSocketCtor: { new (url: string, protocol?: string | string[]): WebSocket }
  binaryType?: 'blob' | 'arraybuffer'
  socketEvents = new EventEmitter()

  private _output: Subject<T>
  private connected = false

  constructor(
    private readonly url: string,
    private readonly protocol?: string | string[],
    private readonly onConnectionLost?: () => void,
    private readonly reconnectTimer: number = 1000,
  ) {
    super()
    if (!root.WebSocket) {
      log.ws.error('WebSocket not supported on this plattform')
      throw new Error('no WebSocket constructor can be found')
    }
    this.WebSocketCtor = root.WebSocket
    this._output = new Subject<T>()
    this.url = url

    this.destination = new ReplaySubject()
  }

  resultSelector(e: MessageEvent): T {
    return JSON.parse(e.data) as T
  }

  static create<T>(
    url: string,
    protocol?: string | string[],
    onConnectionLost?: () => void,
    reconnectTimer: number = 1000,
  ): WebSocketSubject<T> {
    return new WebSocketSubject<T>(url, protocol, onConnectionLost, reconnectTimer)
  }

  private _resetState(): void {
    this.socket = undefined

    if (!this.source) {
      this.destination = new ReplaySubject()
    }
    this._output = new Subject<T>()
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
      const msg = (err as any).message
      log.ws.error('WebSocket connection error -- is ActyxOS reachable?', msg)
      try {
        this._output && this._output.error('Cnx error: ' + msg)
      } catch (err) {
        log.ws.error('ERROR WHILE PASSING WEBSOCKET ERROR UP THE CHAIN', err)
      }
    }
    socket.onmessage = onMessage
    socket.onclose = err => {
      // Can be removed, when the hot reconnect is possible
      if (this.connected) {
        if (this.onConnectionLost) {
          this.onConnectionLost()
        }
        this._output && this._output.error('Connection lost with: ' + JSON.stringify(err))
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

  /**
   * create the Socket connection and wire the Subject to it
   */
  private connect(): void {
    const observer = this._output
    try {
      const onmessage = (e: MessageEvent) => {
        const result = tryCatch(this.resultSelector)(e)
        if (result === errorObject) {
          observer.error(errorObject.e)
        } else {
          observer.next(result)
        }
      }
      this.socket = this.createSocket(onmessage)
    } catch (e) {
      log.ws.error('WebSocket not supported on this plattform')
      observer.error(e)
      return
    }

    // create Subscription and close all connection when all subscriptions are gone
    const subscription = new Subscription(() => {
      const socket = this.socket
      this.socket = undefined
      if (socket && socket.readyState === 1) {
        socket.close()
      }
    })

    const existingQueue = this.destination

    this.destination = Subscriber.create(
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
        this._resetState()
      },
      () => {
        this.socket && this.socket.close()
        this._resetState()
      },
    )

    if (existingQueue && existingQueue instanceof ReplaySubject) {
      subscription.add((existingQueue as ReplaySubject<T>).subscribe(this.destination))
    }
  }

  /** Copied from WebSocketSubject.ts */
  _subscribe(subscriber: Subscriber<T>): Subscription {
    const { source, _output } = this
    if (source) {
      return source.subscribe(subscriber)
    }
    if (!this.socket) {
      this.connect()
    }
    const subscription = new Subscription()
    subscription.add(_output.subscribe(subscriber))
    subscription.add(() => {
      const { socket } = this
      if (_output.observers.length === 0) {
        if (socket && socket.readyState === 1) {
          socket.close()
        }
        this._resetState()
      }
    })
    return subscription
  }

  /** Copied from WebSocketSubject.ts */
  unsubscribe(): void {
    const { source, socket } = this
    if (socket && socket.readyState === 1) {
      socket.close()
      this._resetState()
    }
    super.unsubscribe()
    if (!source) {
      this.destination = new ReplaySubject()
    }
  }
}
