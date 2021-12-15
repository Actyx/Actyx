/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { Subject } from '../../node_modules/rxjs'
import log from '../internal_common/log'
import { WebSocketWrapper } from '../internal_common/webSocketWrapper'
import { ActyxOpts, AppManifest } from '../types'
import { getApiLocation, getToken } from './utils'

export const reconnectingWs = <TRequest, TResponse>(
  opts: ActyxOpts,
  manifest: AppManifest,
): WebSocketWrapper<TRequest, TResponse> => {
  return new ReconnectingWs<TRequest, TResponse>(opts, manifest)
}

class ReconnectingWs<TRequest, TResponse> implements WebSocketWrapper<TRequest, TResponse> {
  responses(): Subject<TResponse> {
    return this.responsesInner
  }

  private responsesInner = new Subject<TResponse>()

  private innerSocket?: WebSocketWrapper<TRequest, TResponse>

  private tryConnect: boolean = true

  private pendingRequests: TRequest[] = []

  sendRequest(req: TRequest): void {
    if (!this.tryConnect) {
      throw new Error('WebSocket closed due to call to close()')
    }

    if (!this.innerSocket) {
      this.pendingRequests.push(req)
      return
    }

    this.innerSocket.sendRequest(req)
  }

  close(): void {
    this.tryConnect = false
    this.innerSocket && this.innerSocket.close()
  }

  constructor(private readonly opts: ActyxOpts, private readonly manifest: AppManifest) {
    this.connect().catch((ex) => console.error('WS unavailable', ex))
  }

  async connect(): Promise<void> {
    const apiLocation = getApiLocation(this.opts.actyxHost, this.opts.actyxPort)
    const token = await getToken(this.opts, this.manifest)
    const wsUrl = 'ws://' + apiLocation + '/events'

    const wsUrlAuthed = wsUrl + '?' + token

    this.innerSocket = WebSocketWrapper(wsUrlAuthed, undefined, this.opts.onConnectionLost)

    this.innerSocket.responses().subscribe({
      next: (x) => this.responsesInner.next(x),
      error: (err) => {
        this.innerSocket && this.innerSocket.close()
        this.innerSocket = undefined

        log.ws.error('WS closed due to', err, '-- attempting reconnect!')

        // Switch to a new subject~~
        const oldSubject = this.responsesInner
        this.responsesInner = new Subject<TResponse>()
        oldSubject.error(err)

        this.loopReconnect()
      },
      complete: () => {
        log.ws.debug('Assuming ordinary close of WebSocket')
        this.responsesInner.complete()
        this.tryConnect = false
      },
    })

    for (const req of this.pendingRequests) {
      this.innerSocket.sendRequest(req)
    }
    this.pendingRequests = []
  }

  private async loopReconnect(): Promise<void> {
    while (this.tryConnect) {
      try {
        await this.connect()
        log.ws.info('Successfully reconnected WS')
        return
      } catch (err) {
        log.ws.error('WS reconnect failed', err, 'trying again in a couple seconds')
        await new Promise((resolve) => setTimeout(resolve, 2_000))
      }
    }

    log.ws.debug('Not reconnecting WS, because user requested close')
    return Promise.reject('Websocket closed due to call to close()')
  }
}
