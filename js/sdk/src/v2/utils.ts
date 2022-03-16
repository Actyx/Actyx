/* eslint-disable @typescript-eslint/no-non-null-assertion */
/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */

import fetch from 'cross-fetch'
import { OffsetsResponse } from '../internal_common'
import { decorateEConnRefused } from '../internal_common/errors'
import { log } from '../internal_common/log'
import { ActyxOpts, AppManifest } from '../types'
import { isNode } from '../util'

const defaultApiLocation = (isNode && process.env.AX_STORE_URI) || 'localhost:4454/api/v2'

export const getApiLocation = (host?: string, port?: number) => {
  if (host || port) {
    return (host || 'localhost') + ':' + (port || 4454) + '/api/v2'
  }

  return defaultApiLocation
}

export const getToken = async (opts: ActyxOpts, manifest: AppManifest): Promise<string> => {
  const apiLocation = getApiLocation(opts.actyxHost, opts.actyxPort)
  const authUrl = 'http://' + apiLocation + '/auth'

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

export const checkToken = async (opts: ActyxOpts, token: string): Promise<boolean> => {
  log.actyx.debug('checking token')
  const apiLocation = getApiLocation(opts.actyxHost, opts.actyxPort)
  const url = 'http://' + apiLocation + '/events/offsets'

  const res = await fetch(url, {
    method: 'get',
    headers: {
      Accept: 'application/json',
      Authorization: `Bearer ${token}`,
    },
  })

  if (res.ok) {
    await res.json()
    return true
  }
  if (res.status === 401) {
    const body = await res.json()
    if (body.code === 'ERR_TOKEN_EXPIRED') return false
  }
  throw new Error(`token check inconclusive, status was ${res.status}`)
}

export const v2getNodeId = async (config: ActyxOpts): Promise<string | null> => {
  const path = `http://${getApiLocation(config.actyxHost, config.actyxPort)}/node/id`
  return await fetch(path)
    .then((resp) => {
      // null indicates the endpoint was reachable but did not react with OK response -> probably V1.
      return resp.ok ? resp.text() : null
    })
    .catch((err) => {
      // ECONNREFUSED is probably not a CORS issue, at least...
      if (err.message && err.message.includes('ECONNREFUSED')) {
        throw new Error(decorateEConnRefused(err.message, path))
      }

      log.actyx.info(
        'Attempt to connect to V2 failed with unclear cause. Gonna try go connect to V1 now. Error was:',
        err,
      )
      // HACK: V1 has broken CORS policy, this blocks our request if it reaches the WS port (4243) instead of the default port (4454).
      // So if we got an error, but the error is (probably) not due to the port being closed, we assume: Probably V1.
      // (Would be awesome if JS API gave a clear and proper indication of CORS block...)
      return null
    })
}
type Uptime = {
  secs: number
  nanos: number
}
type NodeInfo = {
  connectedNodes: number
  uptime: Uptime
  version: string
}

export const mkHeaders = (token: string) => ({
  Accept: 'application/json',
  'Content-Type': 'application/json',
  Authorization: `Bearer ${token}`,
})
enum SyncStage {
  WaitingForPeers = 0,
  WaitingForRootMap,
  WaitingForSync,
  InSync,
}
// Wait at most 30 secs after the node's startup time
const NODE_MAX_STARTED_MS = 30_000
// Once probably a first root map was received, wait up to
// `NODE_REPLICATION_WAIT_MS`. After which we yield if the number of streams to
// replicate is below `NODE_REPLICATION_TARGET_THRESHOLD`.
const NODE_REPLICATION_WAIT_MS = 5000
const NODE_REPLICATION_TARGET_THRESHOLD = 3

export const v2WaitForSwarmSync = async (
  config: ActyxOpts,
  token: string,
  getOffsets: () => Promise<OffsetsResponse>,
): Promise<void> => {
  const uri = `http://${getApiLocation(config.actyxHost, config.actyxPort)}/node/info`
  const getInfo: () => Promise<NodeInfo> = () =>
    fetch(uri, {
      method: 'get',
      headers: mkHeaders(token),
    })
      .then((resp) => {
        if (resp.status === 404) {
          throw new Error(
            'The targeted node seems not to support the `/api/v2/node/info` endpoint. Consider updating to the latest version.',
          )
        } else {
          return resp.json().then((i) => i as NodeInfo)
        }
      })
      .catch((err) => {
        if (err.message) {
          throw new Error(decorateEConnRefused(err.message, uri))
        } else {
          throw new Error(
            `Unknown error trying to contact Actyx node, please diagnose manually by trying to reach ${uri} from where this process is running.`,
          )
        }
      })

  const info = await getInfo()
  let firstNodeSeenAt: number | null = null
  let waitingForSyncSince: number | null = null
  let syncStage = SyncStage.WaitingForPeers as SyncStage

  while (info.uptime.secs * 1000 < NODE_MAX_STARTED_MS) {
    const info = await getInfo()
    switch (syncStage) {
      case SyncStage.WaitingForPeers: {
        if (info.connectedNodes === 0) {
          // Wait a bit and retry.
          await new Promise((res) => setTimeout(res, 500))
        } else {
          // First time there are some peers!
          firstNodeSeenAt = Date.now().valueOf()
          syncStage += 1
        }
        break
      }
      case SyncStage.WaitingForRootMap: {
        // TODO: A more robust approach could be to wait for movements in the
        // offset's response with a cap of 20 s or so.
        //
        // Wait at most up to `firstNodeSeenAt + waitForRootMap`:
        // Default for root map update interval is 10 secs. Assuming an equal
        // distribution of the connected nodes', we can approximate how long to
        // avoid (+standard deviation):
        const waitForRootMap = 1e4 / info.connectedNodes + 2890
        if (Date.now() - firstNodeSeenAt! - waitForRootMap < 0) {
          // Wait a bit and retry.
          await new Promise((res) => setTimeout(res, 250))
        } else {
          // We should have seen at least one root map update by now.
          waitingForSyncSince = Date.now()
          syncStage += 1
        }
        break
      }
      case SyncStage.WaitingForSync: {
        const replicationTarget = (await getOffsets()).toReplicate
        const missingTargets = Object.entries(replicationTarget).length
        if (missingTargets == 0) {
          // Node has peers, we waited a bit to get some root updates AND the
          // replication target is empty. Ignition!
          return
        } else if (
          missingTargets < NODE_REPLICATION_TARGET_THRESHOLD &&
          Date.now() - waitingForSyncSince! > NODE_REPLICATION_WAIT_MS
        ) {
          // Don't let a few bad nodes draw us down
          return
        } else {
          // Wait a bit and retry
          await new Promise((res) => setTimeout(res, 250))
          break
        }
      }
      default: {
        return
      }
    }
  }
}
