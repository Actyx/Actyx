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
import { ActyxOpts, AppManifest } from '../types'
import { mkConfig, MultiplexedWebsocket } from './multiplexedWebsocket'
import { checkToken, getToken, getApiLocation } from './utils'

export const makeWsMultiplexerV2 = async (
  config: ActyxOpts,
  token: string,
  manifest: AppManifest,
): Promise<MultiplexedWebsocket> => {
  const apiLocation = getApiLocation(config.actyxHost, config.actyxPort)
  const wsUrl = (tok: string) => `ws://${apiLocation}/events?${tok}`
  const wsConfig = mkConfig(wsUrl(token))

  const closeSubject = new Subject()
  wsConfig.closeObserver = closeSubject
  const openSubject = new Subject()
  wsConfig.openObserver = openSubject
  const ws = new MultiplexedWebsocket(wsConfig)

  closeSubject.subscribe({
    next: () => {
      log.ws.warn('connection to Actyx lost')
      config.onConnectionLost && config.onConnectionLost()
      let renewing = false
      const renewToken = async () => {
        if (renewing) return
        renewing = true
        if (!(await checkToken(config, token))) {
          // token invalid but API working => reauthenticate
          token = await getToken(config, manifest)
          wsConfig.url = wsUrl(token)
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
      config.onConnectionEstablished && config.onConnectionEstablished()
    },
  })

  return ws
}
