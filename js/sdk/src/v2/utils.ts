/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import fetch from 'node-fetch'
import { ActyxOpts, AppManifest } from '../types'
import { isNode } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'

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

export const v2getNodeId = (config: ActyxOpts): Promise<string | null> =>
  fetch(`http://${getApiLocation(config.actyxHost, config.actyxPort)}/node/id`).then(resp => {
    // null indicates the endpoint was reachable but did not react with OK response -> probably V1.
    return resp.ok ? resp.text() : null
  })

export const mkMultiplexer = async (
  manifest: AppManifest,
  config: ActyxOpts,
): Promise<MultiplexedWebsocket> => {
  const apiLocation = getApiLocation(config.actyxHost, config.actyxPort)

  const authUrl = 'http://' + apiLocation + '/auth'

  const wsUrl = 'ws://' + apiLocation + '/events'
  const cAdjusted = {
    ...config,
    url: wsUrl + '?' + (await getToken(authUrl, manifest)),
  }

  const ws = new MultiplexedWebsocket(cAdjusted)

  return ws
}
