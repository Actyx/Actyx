/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import fetch from 'node-fetch'
import { decorateEConnRefused } from '../internal_common/errors'
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

  if (!res.ok) {
    const errResponse = await res.text()
    if (errResponse && errResponse.includes('message')) {
      const errObj = JSON.parse(errResponse)
      throw new Error(errObj.message)
    } else {
      throw new Error(
        `Could not authenticate with server, got status ${res.status}, content ${errResponse}`,
      )
    }
  }

  const jsonContent = await res.json()
  return (jsonContent as { token: string }).token
}

export const v2getNodeId = async (config: ActyxOpts): Promise<string | null> => {
  const path = `http://${getApiLocation(config.actyxHost, config.actyxPort)}/node/id`
  return await fetch(path)
    .then(resp => {
      // null indicates the endpoint was reachable but did not react with OK response -> probably V1.
      return resp.ok ? resp.text() : null
    })
    .catch(err => {
      if (err.message) {
        throw new Error(decorateEConnRefused(err.message, path))
      } else {
        throw new Error(
          `Unknown error trying to contact Actyx node, please diagnose manually by trying to reach ${path} from where this process is running.`,
        )
      }
    })
}

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
