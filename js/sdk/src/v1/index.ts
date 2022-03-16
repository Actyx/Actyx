/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { Subject } from '../../node_modules/rxjs'
import { EventStore } from '../internal_common/eventStore'
import { log } from '../internal_common/log'
import { SnapshotStore } from '../snapshotStore'
import { ActyxOpts } from '../types'
import { mkConfig, MultiplexedWebsocket } from './multiplexedWebsocket'
import { getSourceId, WebsocketEventStore } from './websocketEventStore'
import { WebsocketSnapshotStore } from './websocketSnapshotStore'

export type Ret = {
  eventStore: EventStore
  close: () => void
  sourceId: string
  snapshotStore: SnapshotStore
}

export const mkV1eventStore = async (axOpts: ActyxOpts): Promise<Ret> => {
  const host = axOpts.actyxHost || 'localhost'
  const port = axOpts.actyxPort || 4243

  /** url of the destination */
  const url = 'ws://' + host + ':' + port + '/store_api'
  log.actyx.debug('Trying V1 connection to', url)

  const wsConfig = mkConfig(url)

  const closeSubject = new Subject()
  wsConfig.closeObserver = closeSubject
  const openSubject = new Subject()
  wsConfig.openObserver = openSubject
  const ws = new MultiplexedWebsocket(wsConfig)

  closeSubject.subscribe({
    next: () => {
      log.ws.warn('connection to Actyx lost')
      axOpts.onConnectionLost && axOpts.onConnectionLost()
    },
  })
  openSubject.subscribe({
    next: () => {
      log.ws.info('connection to Actyx established')
      axOpts.onConnectionEstablished && axOpts.onConnectionEstablished()
    },
  })

  const src = await getSourceId(ws)

  console.warn(
    'Note that the Actyx SDK and Pond 3.0 are optimized for Actyx V2, but you are running Actyx V1. Please upgrade as soon as possible.',
  )

  return {
    eventStore: new WebsocketEventStore(ws, src),
    sourceId: src,
    close: () => ws.close(),
    snapshotStore: new WebsocketSnapshotStore(ws),
  }
}
