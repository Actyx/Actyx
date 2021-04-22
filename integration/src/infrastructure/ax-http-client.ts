import fetch, { Response } from 'node-fetch'

export type AppManifest = Readonly<{
  appId: string
  displayName: string
  version: string
  signature?: string
}>

const API_V2_PATH = '/api/v2'
const EVENTS_PATH = '/events'
const AUTH_PATH = '/authenticate'

export const mkEventsPath = (origin: string) => (segment: string): string =>
  origin + API_V2_PATH + EVENTS_PATH + `${segment}`

const mkHeaders = (token: string) => ({
  Accept: 'application/json',
  'Content-Type': 'application/json',
  Authorization: `Bearer ${token}`,
})

/**
 * Request authorization token
 * @param appManifest
 * @param origin http api endpoint origin, i.e. `http://localhost:4454`
 */
export function getToken(appManifest: AppManifest, origin: string): Promise<Response> {
  return fetch(origin + API_V2_PATH + AUTH_PATH, {
    method: 'post',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(appManifest),
  })
}

export function getNodeId(token: string, origin: string): Promise<Response> {
  return fetch(mkEventsPath(origin)('/node_id'), {
    method: 'get',
    headers: mkHeaders(token),
  })
}
