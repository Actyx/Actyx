import { AppManifest } from '@actyx/sdk'
import fetch, { RequestInit, Response } from 'node-fetch'
import { API_V2_PATH, AUTH_SEG } from './const'
import { decodeOrThrow } from './decode-or-throw'
import { ErrorResponse } from './types'

const mkHeaders = (token: string, xndjson?: boolean) => ({
  Accept: xndjson ? 'application/x-ndjson' : 'application/json',
  'Content-Type': 'application/json',
  Authorization: `Bearer ${token}`,
})

/**
 * Request authorization token
 * @param appManifest
 * @param origin http api endpoint origin, i.e. `http://localhost:4454`
 */
export function getToken(appManifest: AppManifest, origin: string): Promise<Response> {
  return fetch(origin + API_V2_PATH + AUTH_SEG, {
    method: 'post',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(appManifest),
  })
}

export type AxHttpClient = Readonly<{
  /**
   * @param path gets appended to http api origin, must start with `/`
   * @param init optional `RequestInit`, if `Authorization` header is added it will overwrite existing
   */
  fetch: (path: string, init?: RequestInit) => Promise<Response>
  post: <T>(path: string, body: T, xndjson?: boolean) => Promise<Response>
  get: (path: string) => Promise<Response>
}>

const validateSuccess = async (response: Response): Promise<Response> => {
  if (response.ok) {
    return Promise.resolve(response)
  } else if ([400, 401, 405, 406].includes(response.status)) {
    const data = await response.json()
    const error = decodeOrThrow(ErrorResponse)(data)
    throw error
  } else {
    throw new Error(`Unknown response status code: ${response.status}. ${response.statusText}`)
  }
}

const fixedTokenClient = (httpOrigin: string) => (token: string) => ({
  fetch: (path: string, init: RequestInit = {}) =>
    fetch(httpOrigin + path, {
      ...init,
      headers: {
        Authorization: `Bearer ${token}`,
        ...init.headers,
      },
    }).then(validateSuccess),

  post: <T>(path: string, body: T, xndjson?: boolean) =>
    fetch(httpOrigin + path, {
      method: 'post',
      headers: mkHeaders(token, xndjson),
      body: JSON.stringify(body),
    }).then(validateSuccess),

  get: (path: string) =>
    fetch(httpOrigin + path, {
      method: 'get',
      headers: mkHeaders(token),
    }).then(validateSuccess),
})

// TODO: if reused in js sdk on expired token retry once
export const mkAuthHttpClient = (manifest: AppManifest) => (
  httpOrigin: string,
): Promise<AxHttpClient> =>
  getToken(manifest, httpOrigin)
    .then((authResponse) => authResponse.json())
    .then((x) => x.token)
    .then(fixedTokenClient(httpOrigin))
