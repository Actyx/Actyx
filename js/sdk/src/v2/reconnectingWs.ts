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

export const reconnectingWs = async <TRequest, TResponse>(
  opts: ActyxOpts,
  manifest: AppManifest,
): Promise<WebSocketWrapper<TRequest, TResponse>> => {
  const ws = new ReconnectingWs<TRequest, TResponse>(opts, manifest)
  await ws.connect()
  return ws
}

export class ReconnectingWs<TRequest, TResponse> implements WebSocketWrapper<TRequest, TResponse> {
  readonly responses: Subject<TResponse> = new Subject()

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
    this.connect()
  }

  async connect(): Promise<void> {
    const apiLocation = getApiLocation(this.opts.actyxHost, this.opts.actyxPort)
    const token = await getToken(this.opts, this.manifest)
    const wsUrl = 'ws://' + apiLocation + '/events'

    const wsUrlAuthed = wsUrl + '?' + token

    this.innerSocket = WebSocketWrapper(wsUrlAuthed, undefined)

    this.innerSocket.responses.subscribe({
      next: x => this.responses.next(x),
      error: async err => {
        this.innerSocket = undefined

        log.ws.error('WS closed due to', err, 'attempting reconnect')

        await this.loopReconnect()
      },
      complete: () => {
        log.ws.debug('Assuming ordinary close of WebSocket')
      },
    })
  }

  private async loopReconnect(): Promise<void> {
    while (this.tryConnect) {
      try {
        await this.connect()
      } catch (err) {
        log.ws.error('WS reconnect failed', err, 'trying again in a couple seconds')
        await new Promise(resolve => setTimeout(resolve, 2_000))
      }
    }

    log.ws.debug('Not reconnecting WS, because user requested close')
  }
}
