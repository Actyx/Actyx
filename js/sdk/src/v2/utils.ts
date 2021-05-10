/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import fetch from 'node-fetch'
import { NodeId } from '../types'
import { isNode } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { Event, Events, WsStoreConfig } from './types'
import { getNodeId } from './websocketEventStore'

const defaultConfig: WsStoreConfig = {
  url: (isNode && process.env.AX_STORE_URI) || 'ws://localhost:4454/api/v2/events',
}

export const extendDefaultWsStoreConfig = (overrides: Partial<WsStoreConfig> = {}) => ({
  ...defaultConfig,
  ...overrides,
})

const getToken = async (authUrl: string): Promise<string> => {
  const res = await fetch(authUrl, {
    method: 'post',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      appId: 'com.example.dev-Pond',
      displayName: 'Pond dev',
      version: '1.0.0',
    }),
  })

  const jsonContent = await res.json()
  return (jsonContent as { token: string }).token
}

export const mkMultiplexer = async (
  config: Partial<WsStoreConfig> = {},
): Promise<[MultiplexedWebsocket, NodeId]> => {
  const c: WsStoreConfig = extendDefaultWsStoreConfig(config)

  // FIXME obviously...
  const authUrl = c.url.replace('ws://', 'http://').replace('/events', '/authenticate')

  const cAdjusted = {
    ...c,
    url: c.url + '?' + (await getToken(authUrl)),
  }

  const ws = new MultiplexedWebsocket(cAdjusted)

  const nodeId = await getNodeId(ws)

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
