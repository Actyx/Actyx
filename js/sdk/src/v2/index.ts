/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
export { v2getNodeId } from './utils'
export { WebsocketEventStore as WebsocketEventStoreV2 } from './websocketEventStore'

import { WebSocketWrapper } from '../internal_common/webSocketWrapper'
import { ActyxOpts, AppManifest } from '../types'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { reconnectingWs } from './reconnectingWs'
import { getApiLocation } from './utils'

export const makeWsMultiplexerV2 = async (
  config: ActyxOpts,
  token: string,
  manifest: AppManifest,
): Promise<MultiplexedWebsocket> => {
  if (config.automaticReconnect) {
    return new MultiplexedWebsocket(await reconnectingWs(config, manifest))
  }

  const apiLocation = getApiLocation(config.actyxHost, config.actyxPort)

  const wsUrl = 'ws://' + apiLocation + '/events'

  const ws = new MultiplexedWebsocket(
    WebSocketWrapper(wsUrl + '?' + token, undefined, config.onConnectionLost),
  )

  return ws
}
