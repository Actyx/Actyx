import { RequestInit } from 'node-fetch'
import {
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
    describe('user gets ERR_BAD_REQUEST', () => {
      it.each([...postEndPoints])(
        'should return error if body request is malformed for %p',
        expectErr('ERR_BAD_REQUEST', {
          headers: {
            Accept: 'application/json, application/x-ndjson',
            'Content-Type': 'application/json',
          },
          method: 'post',
          body: JSON.stringify({ 'malformed-body-key': 'malformed-body-value' }),
        }),
      )
    })

    describe('user gets ERR_METHOD_NOT_ALLOWED', () => {
      const mk = (method: 'post' | 'get') =>
        expectErr('ERR_METHOD_NOT_ALLOWED', {
          method,
          headers: {
            Accept: 'application/json, application/x-ndjson',
            'Content-Type': 'application/json',
          },
        })

      it.each(getEndPoints)(
        'should return error if endpoint method is GET and instead user uses POST for %p',
        mk('post'),
      )

      it.each(postEndPoints)(
        'should return error if endpoint method is POST and instead user uses GET for %p',
        mk('get'),
      )
    })

    describe('user gets ERR_NOT_ACCEPTABLE', () => {
      const mk = (method: 'post' | 'get') =>
        expectErr('ERR_METHOD_NOT_ALLOWED', {
          method,
          headers: {
            Accept: 'invalid',
            'Content-Type': 'application/json',
          },
        })

      it.each([...getEndPoints])(
        'should return error if server cannot produce a response matching the list of acceptable values defined in the request for %p',
        mk('get'),
      )
      it.each([...postEndPoints])(
        'should return error if server cannot produce a response matching the list of acceptable values defined in the request for %p',
        mk('post'),
      )
    })
  })
})
