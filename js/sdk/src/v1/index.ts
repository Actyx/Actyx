/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { EventStore } from '../internal_common/eventStore'
import { log } from '../internal_common/log'
import { WebSocketWrapper } from '../internal_common/webSocketWrapper'
import { ActyxOpts } from '../types'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { getSourceId, WebsocketEventStore } from './websocketEventStore'

export type Ret = {
  eventStore: EventStore
  close: () => void
  sourceId: string
}

export const mkV1eventStore = async (axOpts: ActyxOpts): Promise<Ret> => {
  const host = axOpts.actyxHost || 'localhost'
  const port = axOpts.actyxPort || 4243

  /** url of the destination */
  const url = 'ws://' + host + ':' + port + '/store_api'

  /** Hook, when the connection to the store is closed */
  const onStoreConnectionClosed = axOpts.onConnectionLost

  const reconnectTimer = axOpts.automaticReconnect ? 2_000 : undefined

  log.actyx.debug('Trying V1 connection to', url)

  const ws = new MultiplexedWebsocket(
    WebSocketWrapper(url, undefined, onStoreConnectionClosed, reconnectTimer),
  )
  const src = await getSourceId(ws)

  console.warn(
    'Note that the Actyx SDK and Pond 3.0 are optimized for Actyx V2, but you are running Actyx V1. Please upgrade as soon as possible.',
  )

  return {
    eventStore: new WebsocketEventStore(ws, src),
    sourceId: src,
    close: () => ws.close(),
  }
}
