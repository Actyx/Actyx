/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
export { v2getNodeId } from './utils'
export { WebsocketEventStore as WebsocketEventStoreV2 } from './websocketEventStore'

import { Subject, takeUntil } from '../../node_modules/rxjs'
import log from '../internal_common/log'
import { getInfo } from '../node-info'
import { ActyxOpts, AppManifest } from '../types'
import { massageError } from '../util/error'
import { mkConfig, MultiplexedWebsocket } from './multiplexedWebsocket'
import { checkToken, getToken, getApiLocation } from './utils'

export const makeWsMultiplexerV2 = async (
  config: ActyxOpts,
  token: string,
  manifest: AppManifest,
): Promise<[MultiplexedWebsocket, [string, string]]> => {
  const apiLocation = getApiLocation(config.actyxHost, config.actyxPort)
  const wsUrl = (tok: string) => `ws://${apiLocation}/events?${tok}`
  const wsConfig = mkConfig(wsUrl(token))
  const getVersion = async (token: string) => (await getInfo(config)(token, -1)).semVer()
  const version = await getVersion(token)
  const tokVer: [string, string] = [token, version]

  const closeSubject = new Subject()
  wsConfig.closeObserver = closeSubject
  const openSubject = new Subject()
  wsConfig.openObserver = openSubject
  const ws = new MultiplexedWebsocket(wsConfig)

  let disconnected = false

  log.ws.info('websocket initiated')
  closeSubject.subscribe({
    next: (err) => {
      if (disconnected) {
        return
      }
      disconnected = true
      log.ws.warn('connection to Actyx lost', massageError(err))
      config.onConnectionLost && config.onConnectionLost()
      let renewing = false
      const renewToken = async () => {
        if (renewing) return
        renewing = true
        if (!(await checkToken(config, tokVer[0]))) {
          // tokVer[0] invalid but API working => reauthenticate
          tokVer[0] = await getToken(config, manifest)
          wsConfig.url = wsUrl(tokVer[0])
          tokVer[1] = await getVersion(tokVer[0])
          ws.close() // this disposes of the internal WebSocketSubject
          ws.request('wake up')
        }
      }
      ws.errors()
        .pipe(takeUntil(openSubject))
        .forEach(() => renewToken().catch(() => (renewing = false)))
    },
  })
  openSubject.subscribe({
    next: () => {
      log.ws.info('connection to Actyx established')
      disconnected = false
      config.onConnectionEstablished && config.onConnectionEstablished()
    },
  })

  return [ws, tokVer]
}
