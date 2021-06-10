import fetch from 'node-fetch'
import {
  getToken,
  mkEventsPath,
  trialManifest,
  API_V2_PATH,
  EVENTS_PATH,
  OFFSETS_SEG,
  AUTH_SEG,
} from '../../http-client'
import WebSocket from 'ws'
import { run } from '../../util'
import { createTestNodeLocal } from '../../test-node-factory'
import { AppManifest } from '@actyx/pond'

const UNAUTHORIZED_TOKEN =
  'AAAAWaZnY3JlYXRlZBsABb3ls11m8mZhcHBfaWRyY29tLmV4YW1wbGUubXktYXBwZmN5Y2xlcwBndmVyc2lvbmUxLjAuMGh2YWxpZGl0eRkBLGlldmFsX21vZGX1AQv+4BIlF/5qZFHJ7xJflyew/CnF38qdV1BZr/ge8i0mPCFqXjnrZwqACX5unUO2mJPsXruWYKIgXyUQHwKwQpzXceNzo6jcLZxvAKYA05EFDnFvPIRfoso+gBJinSWpDQ=='

const getOffsets = (httpApi: string, authHeaderValue?: string) =>
  fetch(httpApi + API_V2_PATH + EVENTS_PATH + OFFSETS_SEG, {
    method: 'get',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
      ...(authHeaderValue ? { Authorization: authHeaderValue } : {}),
    },
  })

describe('auth http', () => {
  const signedManifest: AppManifest = {
    appId: 'com.actyx.auth-test',
    displayName: 'auth test app',
    version: 'v0.0.1',
    signature:
      'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
  }

  it('should get token for signed manifest', () =>
    run((httpApi) =>
      getToken(signedManifest, httpApi)
        .then((x) => x.json())
        .then((x) =>
          expect(x).toEqual({
            token: expect.any(String),
          }),
        ),
    ))

  it('should fail to get token for falsified manifest', () =>
    run((httpApi) =>
      getToken({ ...signedManifest, version: '1' }, httpApi)
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_MANIFEST_INVALID',
            message:
              'Invalid manifest. Failed to validate app manifest. Invalid signature for provided input.',
          }),
        ),
    ))

  it('should fail for malformed requests', () =>
    run((httpApi) =>
      fetch(httpApi + API_V2_PATH + AUTH_SEG, {
        method: 'post',
        body: JSON.stringify({ malformed: true }),
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
        },
      })
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_BAD_REQUEST',
            message: 'Invalid request. data did not match any variant of untagged enum AppManifest',
          }),
        ),
    ))

  it('should fail when the manifest is invalid', () =>
    run((httpApi) =>
      fetch(httpApi + API_V2_PATH + AUTH_SEG, {
        method: 'post',
        body: JSON.stringify({
          appId: 'com.actyx.my-app',
          displayName: 'Mine!',
          version: '0.8.5',
          signature:
            'v2tzaWdfdmVyc2lvbgBtZGV2X3NpZ25hdHVyZXhYZ0JGTTgyZVpMWTdJQzhRbmFuVzFYZ0xrZFRQaDN5aCtGeDJlZlVqYm9qWGtUTWhUdFZNRU9BZFJaMVdTSGZyUjZUOHl1NEFKdFN5azhMbkRvTVhlQnc9PWlkZXZQdWJrZXl4LTBuejFZZEh1L0pEbVM2Q0ltY1pnT2o5WTk2MHNKT1ByYlpIQUpPMTA3cVcwPWphcHBEb21haW5zgmtjb20uYWN0eXguKm1jb20uZXhhbXBsZS4qa2F4U2lnbmF0dXJleFg4QmwzekNObm81R2JwS1VvYXRpN0NpRmdyMEtHd05IQjFrVHdCVkt6TzlwelcwN2hGa2tRK0dYdnljOVFhV2hIVDVhWHp6TyttVnJ4M2VpQzdUUkVBUT09/w==',
        }),
        headers: {
          Accept: 'application/json',
          'Content-Type': 'application/json',
        },
      })
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((json) =>
          expect(json).toEqual({
            code: 'ERR_MANIFEST_INVALID',
            message:
              'Invalid manifest. Failed to validate app manifest. Invalid signature for provided input.',
          }),
        ),
    ))

  it('should fail when token not authorized', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Bearer ' + UNAUTHORIZED_TOKEN)
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_TOKEN_UNAUTHORIZED',
            message: 'Unauthorized token.',
          }),
        ),
    ))

  it('should fail when auth header has wrong value', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Foo bar')
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_UNSUPPORTED_AUTH_TYPE',
            message: 'Unsupported authentication type \'Foo\'. Only "Bearer" is supported.',
          }),
        ),
    ))

  it('should fail when token is invalid', () =>
    run((httpApi) =>
      getOffsets(httpApi, 'Bearer invalid')
        .then((resp) => {
          expect(resp.status).toEqual(400)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_TOKEN_INVALID',
            message:
              "Invalid token: 'invalid'. Cannot parse token bytes. Please provide a valid bearer token.",
          }),
        ),
    ))

  it('should fail when authorization header is missing', () =>
    run((httpApi) =>
      getOffsets(httpApi)
        .then((resp) => {
          expect(resp.status).toEqual(401)
          return resp.json()
        })
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_MISSING_AUTH_HEADER',
            message: '"Authorization" header is missing.',
          }),
        ),
    ))

  it('should fail for a valid token when node is cycled', async () => {
    const nodeName = 'es-auth'
    let testNode = await createTestNodeLocal(nodeName)
    const token = await getToken(trialManifest, testNode._private.httpApiOrigin)
      .then((x) => x.json())
      .then((x) => x.token)
    const offsets = (origin: string) => getOffsets(origin, 'Bearer ' + token)

    // assert we can access event service
    const response = await offsets(testNode._private.httpApiOrigin).then((resp) => resp.json())
    expect(response).toEqual({ present: expect.any(Object), toReplicate: expect.any(Object) })
    await testNode._private.shutdown()

    // start the node again and assert that we can't reuse previous token
    testNode = await createTestNodeLocal(nodeName, true)
    const result = await offsets(testNode._private.httpApiOrigin).then((resp) => {
      expect(resp.status).toEqual(401)
      return resp.json()
    })
    expect(result).toEqual({ code: 'ERR_TOKEN_EXPIRED', message: 'Expired token.' })
  })

  // TODO: test expired token response, for that node's AX_API_TOKEN_VALIDITY
  // env value needs to be set to 1s. What is the best way to do so.
})

describe('auth ws', () => {
  const mkWs = (path: string, f: (ws: WebSocket, resolve: () => void) => void): Promise<void[]> =>
    run((httpApi) => {
      const ws = new WebSocket(httpApi + mkEventsPath(path))
      return new Promise<void>((resolve) => {
        f(ws, resolve)
      })
    })

  const expectFailure = (path: string, status: number): Promise<void[]> =>
    mkWs(path, (ws, resolve) => {
      ws.on('error', (x) => {
        expect(x.message).toEqual(`Unexpected server response: ${status}`)
        resolve()
      })
    })

  it('should fail when token is missing', () => expectFailure('', 401))

  it('should fail when token is not authorized', () => expectFailure(`?${UNAUTHORIZED_TOKEN}`, 401))

  it('should fail when using wrong path', () =>
    expectFailure(`/wrong_path?token-does-not-matter`, 404))

  it('should get token for a trial manifest and successfully use it', () =>
    run((httpApi) =>
      getToken(trialManifest, httpApi)
        .then((authResponse) => authResponse.json())
        .then((x) => {
          const ws = new WebSocket(httpApi + mkEventsPath(`?${x.token}`))
          const message = {
            type: 'request',
            serviceId: 'offsets',
            requestId: 1,
            payload: null,
          }
          const responses: unknown[] = []
          return new Promise<void>((resolve) => {
            ws.on('message', (x) => {
              responses.push(JSON.parse(x.toString()))
              if (responses.length === 2) {
                expect(responses).toEqual([
                  {
                    type: 'next',
                    requestId: 1,
                    payload: [{ present: expect.any(Object), toReplicate: expect.any(Object) }],
                  },
                  { type: 'complete', requestId: 1 },
                ])
                ws.terminate()
                resolve()
              }
            })
            ws.onopen = () => ws.send(JSON.stringify(message))
          })
        }),
    ))
})
