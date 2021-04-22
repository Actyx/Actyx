import { AxiosError } from 'axios'
import { ErrorResponse } from '../../event-service-types'
import {
  httpClient,
  httpClientInvalidAccept,
  httpClientInvalidToken,
  httpClientNoHeaders,
} from '../../httpClient'

const postEndPoints = [['publish'], ['query'], ['subscribe_monotonic'], ['subscribe']]
const getEndPoints = [['node_id'], ['offsets']]

const allEndPoints = [...postEndPoints, ...getEndPoints]

describe('event service', () => {
  describe('errors for endpoints', () => {
    describe('user gets ERR_MALFORMED_REQUEST_SYNTAX', () => {
      it.each([...postEndPoints])(
        'should return error if body request is malformed for %p',
        async (path) => {
          await httpClient
            .post(path, { 'malformed-body-key': 'malformed-body-value' })
            .catch((error: AxiosError<ErrorResponse>) =>
              expect(error).toMatchErrorMalformedRequestSytantax(),
            )
        },
      )
    })

    describe('user gets ERR_MISSING_AUTH_HEADER', () => {
      it.each([...allEndPoints])(
        'should return error if Authorization header is missing for %p',
        async (path) => {
          await httpClientNoHeaders
            .get(path)
            .catch((error: AxiosError<ErrorResponse>) =>
              expect(error).toMatchErrorMissingAuthHeader(),
            )
        },
      )
    })

    describe('user gets ERR_METHOD_NOT_ALLOWED', () => {
      it.each(getEndPoints)(
        'should return error if endpoint method is GET and instead user uses POST for %p',
        async (path) => {
          await httpClient
            .post(path)
            .catch((error: AxiosError<ErrorResponse>) =>
              expect(error).toMatchErrorMethodNotAllowed(),
            )
        },
      )

      it.each(postEndPoints)(
        'should return error if endpoint method is POST and instead user uses GET for %p',
        async (path) => {
          await httpClient
            .get(path)
            .catch((error: AxiosError<ErrorResponse>) =>
              expect(error).toMatchErrorMethodNotAllowed(),
            )
        },
      )
    })

    describe('user gets ERR_NOT_ACCEPTABLE', () => {
      it.each([...getEndPoints])(
        'should return error if server cannot produce a response matching the list of acceptable values defined in the request for %p',
        async (path) => {
          await httpClientInvalidAccept
            .get(path)
            .catch((error: AxiosError<ErrorResponse>) => expect(error).toMatchErrorNotAcceptable())
        },
      )
      it.each([...postEndPoints])(
        'should return error if server cannot produce a response matching the list of acceptable values defined in the request for %p',
        async (path) => {
          await httpClientInvalidAccept
            .post(path)
            .catch((error: AxiosError<ErrorResponse>) => expect(error).toMatchErrorNotAcceptable())
        },
      )
    })

    describe('user gets ERR_TOKEN_INVALID', () => {
      it.each([...allEndPoints])(
        'should return error if user does not provide a valid bearer token for %p',
        async (path) => {
          await httpClientInvalidToken
            .get(path)
            .catch((error: AxiosError<ErrorResponse>) => expect(error).toMatchErrorTokenInvalid())
        },
      )
    })
  })
})
