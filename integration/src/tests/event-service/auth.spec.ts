import fetch from 'node-fetch'
import { getToken, getNodeId, mkEventsPath } from '../../infrastructure/ax-http-client'
import { runConcurrentlyOnAllSameLogic } from '../../infrastructure/hosts'
import WebSocket from 'ws'

const UNAUTHORIZED_TOKEN =
  'AAAAWaZnY3JlYXRlZBsABb3ls11m8mZhcHBfaWRyY29tLmV4YW1wbGUubXktYXBwZmN5Y2xlcwBndmVyc2lvbmUxLjAuMGh2YWxpZGl0eRkBLGlldmFsX21vZGX1AQv+4BIlF/5qZFHJ7xJflyew/CnF38qdV1BZr/ge8i0mPCFqXjnrZwqACX5unUO2mJPsXruWYKIgXyUQHwKwQpzXceNzo6jcLZxvAKYA05EFDnFvPIRfoso+gBJinSWpDQ=='

const trialManifest = {
  appId: 'com.example.my-app',
  displayName: 'My Example App',
  version: '1.0.0',
}

const run = <T>(f: (httpApi: string) => Promise<T>): Promise<T[]> =>
  runConcurrentlyOnAllSameLogic((node) => {
    const httpApi = new URL(node._private.apiEvent).origin
    return f(httpApi)
  })

describe('auth http', () => {
  it('should get token for a trial manifest and successfully use it', () =>
    run((httpApi) =>
      getToken(trialManifest, httpApi)
        .then((authResponse) => authResponse.json())
        .then((x) => {
          expect(x).toEqual({ token: expect.any(String) })
          return getNodeId(x.token, httpApi)
        })
        .then((nodeIdResponse) => nodeIdResponse.json())
        .then((x) => expect(x).toEqual({ nodeId: expect.any(String) })),
    ))

  it('should fail when token not authorized', () =>
    run((httpApi) =>
      getNodeId(UNAUTHORIZED_TOKEN, httpApi)
        .then((nodeIdResponse) => nodeIdResponse.json())
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_TOKEN_UNAUTHORIZED',
            message: 'Unauthorized token.',
          }),
        ),
    ))

  const getId = (httpApi: string, authHeaderValue?: string) =>
    fetch(mkEventsPath(httpApi)('/node_id'), {
      method: 'get',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
        ...(authHeaderValue ? { Authorization: authHeaderValue } : {}),
      },
    })

  it('should fail when auth header has wrong value', () =>
    run((httpApi) =>
      getId(httpApi, 'Foo bar')
        .then((x) => x.json())
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_WRONG_AUTH_TYPE',
            message: 'Unsupported authentication type \'Foo\'. Only "Bearer" is supported.',
          }),
        ),
    ))

  it('should fail when token is invalid', () =>
    run((httpApi) =>
      getId(httpApi, 'Bearer invalid')
        .then((x) => x.json())
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
      getId(httpApi)
        .then((x) => x.json())
        .then((x) =>
          expect(x).toEqual({
            code: 'ERR_MISSING_AUTH_HEADER',
            message: '"Authorization" header is missing.',
          }),
        ),
    ))

  // TODO: test expired token response, for that node's AX_API_TOKEN_VALIDITY
  // env value needs to be set to 1s. What is the best way to do so.
})

describe('auth ws', () => {
  const mkWs = (path: string, f: (ws: WebSocket, resolve: () => void) => void): Promise<void[]> =>
    run((httpApi) => {
      const ws = new WebSocket(mkEventsPath(httpApi)(path))
      return new Promise<void>((resolve) => {
        f(ws, resolve)
      })
    })

  const expectFailure = (path: string): Promise<void[]> =>
    mkWs(path, (ws, resolve) => {
      ws.on('error', (x) => {
        expect(x.message).toEqual('Unexpected server response: 401')
        resolve()
      })
    })

  it('should fail when token is missing', () => expectFailure(''))

  it('should fail when token is not authorized', () => expectFailure(`?${UNAUTHORIZED_TOKEN}`))

  it('should fail when using wrong path', () => expectFailure(`/wrong_path?token-does-not-matter`))

  it('should get token for a trial manifest and successfully use it', () =>
    run((httpApi) =>
      getToken(trialManifest, httpApi)
        .then((authResponse) => authResponse.json())
        .then((x) => {
          const ws = new WebSocket(mkEventsPath(httpApi)(`?${x.token}`))
          const message = {
            type: 'request',
            serviceId: 'node_id',
            requestId: 1,
            payload: null,
          }
          const responses: unknown[] = []
          return new Promise<void>((resolve) => {
            ws.on('message', (x) => {
              responses.push(JSON.parse(x.toString()))
              if (responses.length === 2) {
                expect(responses).toEqual([
                  { type: 'next', requestId: 1, payload: { nodeId: expect.any(String) } },
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
