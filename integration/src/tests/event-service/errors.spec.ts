import { RequestInit } from 'node-fetch'
import {
  ErrorCode,
  mkEventsPath,
  mkTrialHttpClient,
  NODE_ID_SEG,
  OFFSETS_SEG,
  PUBLISH_SEG,
  QUERY_SEG,
  SUBSCRIBE_MONOTONIC_SEG,
  SUBSCRIBE_SEG,
} from '../../http-client'
import { run } from '../../util'

const postEndPoints = [[PUBLISH_SEG], [QUERY_SEG], [SUBSCRIBE_MONOTONIC_SEG], [SUBSCRIBE_SEG]]
const getEndPoints = [[NODE_ID_SEG], [OFFSETS_SEG]]

const expectErr = (errorCode: string, req: RequestInit) => async (segment: string) => {
  const runTest = async (httpEndpoint: string) => {
    const client = await mkTrialHttpClient(httpEndpoint)

    const response = client.fetch(mkEventsPath(segment), req)

    await expect(response).rejects.toEqual({
      code: errorCode,
      message: expect.any(String),
    })
  }

  await run(runTest)
}

// TODO: move tests to dedicated endpoint tests and assert messages
describe('event service', () => {
  describe('errors for endpoints', () => {
    describe('common errors', () => {
      // TODO:
      // - ERR_APP_UNAUTHORIZED
      // - ERR_APP_UNAUTHENTICATED
      const errors: [string, ErrorCode, RequestInit, ('get' | 'post')?][] = [
        [
          'the request body is malformed',
          'ERR_BAD_REQUEST',
          {
            headers: {
              Accept: 'application/json, application/x-ndjson',
              'Content-Type': 'application/json',
            },
            body: JSON.stringify({ 'malformed-body-key': 'malformed-body-value' }),
          },
          'post',
        ],
        [
          'the request method is not allowed',
          'ERR_METHOD_NOT_ALLOWED',
          {
            method: 'get',
          },
          'post',
        ],
        [
          'the request method is not allowed',
          'ERR_METHOD_NOT_ALLOWED',
          {
            method: 'post',
          },
          'get',
        ],
        [
          'the server cannot produce a response matching the list of acceptable content types',
          'ERR_NOT_ACCEPTABLE',
          { headers: { Accept: 'invalid' } },
        ],
        [
          'the server does not support the provided authorization type',
          'ERR_UNSUPPORTED_AUTH_TYPE',
          { headers: { Authorization: 'Bierer 123' } },
        ],
      ]
      for (const [msg, code, reqInit, method] of errors) {
        describe(`should return ${code} if ${msg}`, () => {
          if (!method || method === 'get') {
            for (const [getEndpoint] of getEndPoints) {
              it(`GET ${getEndpoint}`, () =>
                expectErr(code, { method: 'get', ...reqInit })(getEndpoint))
            }
          }
          if (!method || method === 'post') {
            for (const [postEndpoint] of postEndPoints) {
              it(`POST ${postEndpoint}`, () =>
                expectErr(code, { method: 'post', ...reqInit })(postEndpoint))
            }
          }
        })
      }
    })
  })
})
