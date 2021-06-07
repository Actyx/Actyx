/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import fetch from 'node-fetch'
import { ActyxOpts, AppManifest, NodeId } from '../types'
import { isNode } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { Event, Events } from './types'

const defaultApiLocation = (isNode && process.env.AX_STORE_URI) || 'localhost:4454/api/v2'

const getApiLocation = (host?: string, port?: number) => {
  if (host || port) {
    return (host || 'localhost') + ':' + (port || 4454) + '/api/v2'
  }

  return defaultApiLocation
}

const getToken = async (authUrl: string, manifest: AppManifest): Promise<string> => {
  const res = await fetch(authUrl, {
    method: 'post',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(manifest),
  })

  const jsonContent = await res.json()
  return (jsonContent as { token: string }).token
}

export const mkMultiplexer = async (
  manifest: AppManifest,
  config: ActyxOpts,
): Promise<[MultiplexedWebsocket, NodeId]> => {
  const apiLocation = getApiLocation(config.actyxHost, config.actyxPort)

  // FIXME obviously...
  const authUrl = 'http://' + apiLocation + '/auth'

  const wsUrl = 'ws://' + apiLocation + '/events'
  const cAdjusted = {
    ...config,
    url: wsUrl + '?' + (await getToken(authUrl, manifest)),
  }

  const ws = new MultiplexedWebsocket(cAdjusted)

  const nodeId = await fetch(`http://${apiLocation}/node/id`).then(resp => resp.text())

  return [ws, nodeId]
}

// Partition an unordered batch of events into several, where each is internally ordered.
// Will not copy any data if the whole input batch is already sorted.
export const intoOrderedChunks = (batch: Events) => {
  if (batch.length < 2) {
    return [batch]
  }

  const orderedBatches: Events[] = []

  let prev = batch[0]
  let start = 0

  for (let i = 1; i < batch.length; i++) {
    const evt = batch[i]

    if (Event.ord.compare(prev, evt) > 0) {
      orderedBatches.push(batch.slice(start, i))
      start = i
    }

    prev = evt
  }

  if (start === 0) {
    // Everything was sorted already
    orderedBatches.push(batch)
  } else {
    orderedBatches.push(batch.slice(start))
  }

  return orderedBatches
}
