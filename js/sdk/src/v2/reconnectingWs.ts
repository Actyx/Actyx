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
  responses: Promise<Subject<TResponse>>

  private innerSocket?: WebSocketWrapper<TRequest, TResponse>

  private tryConnect: boolean = true

  sendRequest(req: TRequest): void {
    if (!this.innerSocket) {
      throw new Error('WebSocket currently is closed')
    }

    this.innerSocket.sendRequest(req)
  }

  close(): void {
    this.tryConnect = false
    this.innerSocket && this.innerSocket.close()
  }

  constructor(private readonly opts: ActyxOpts, private readonly manifest: AppManifest) {
    this.responses = this.connect()
  }

  async connect(): Promise<Subject<TResponse>> {
    const apiLocation = getApiLocation(this.opts.actyxHost, this.opts.actyxPort)
    const token = await getToken(this.opts, this.manifest)
    const wsUrl = 'ws://' + apiLocation + '/events'

    const wsUrlAuthed = wsUrl + '?' + token

    this.innerSocket = WebSocketWrapper(wsUrlAuthed, undefined)

    const innerRes = await this.innerSocket.responses

    innerRes.subscribe({
      error: err => {
        this.innerSocket && this.innerSocket.close()
        this.innerSocket = undefined

        log.ws.error('WS closed due to', err, 'attempting reconnect')

        this.responses = this.loopReconnect()
      },
      complete: () => {
        log.ws.debug('Assuming ordinary close of WebSocket')
      },
    })

    return innerRes
  }

  private async loopReconnect(): Promise<Subject<TResponse>> {
    while (this.tryConnect) {
      try {
        const r = await this.connect()
        log.ws.info('Successfully reconnected WS')
        return r
      } catch (err) {
        log.ws.error('WS reconnect failed', err, 'trying again in a couple seconds')
        await new Promise(resolve => setTimeout(resolve, 20_000))
      }
    }

    log.ws.debug('Not reconnecting WS, because user requested close')
    return Promise.reject('Websocket closed due to call to close()')
  }
}