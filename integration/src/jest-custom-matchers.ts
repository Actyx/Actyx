import { AxiosError } from 'axios'
import { ErrorResponse } from './event-service-types'

/* eslint-disable @typescript-eslint/no-explicit-any */
const hasPropertyResponse = (data: any, propery: string, value?: string | number) => {
  if (value) {
    return data && propery in data && data[propery] === value
  } else {
    return data && propery in data
  }
}

const validateResponseError = (response: any, codeExpected: string) => {
  const hasMessage = hasPropertyResponse(response, 'message')
  const hasCode = hasPropertyResponse(response, 'code', codeExpected)
  const pass = hasCode && hasMessage
  const message = () =>
    `Expected code was: ${codeExpected} instead got: ${response.code}, message was ${
      hasMessage ? 'found' : 'not found'
    }, response was: ${JSON.stringify(response)}`
  return {
    message,
    pass,
  }
}

const mkResultForEventService = (
  errorResp: AxiosError<ErrorResponse>,
  expectedStatus: number,
  expectedCode: string,
) => {
  const passStatus = hasPropertyResponse(errorResp.response, 'status', expectedStatus)
  const passCode = hasPropertyResponse(errorResp.response?.data, 'code', expectedCode)
  const message = () => `Expected status was ${expectedStatus} and expected code was: ${expectedCode},\
  instead got status: ${errorResp.response?.status} and code: ${errorResp.response?.data?.code},\
  response was: ${JSON.stringify(errorResp)}`

  return {
    message,
    pass: passCode && passStatus,
  }
}

expect.extend({
  toMatchErrNodeUnreachable(response: any): jest.CustomMatcherResult {
    const { message, pass } = validateResponseError(response, 'ERR_NODE_UNREACHABLE')
    return {
      message,
      pass,
    }
  },
  toMatchErrInvalidInput(response: any): jest.CustomMatcherResult {
    const { message, pass } = validateResponseError(response, 'ERR_INVALID_INPUT')
    return {
      message,
      pass,
    }
  },
  toMatchCodeOk(response: any): jest.CustomMatcherResult {
    const expectedCode = 'OK'
    const pass = hasPropertyResponse(response, 'code', expectedCode)
    const message = () =>
      `Expected code was: ${expectedCode}, instead got: ${
        response.code
      }, response was: ${JSON.stringify(response)}`
    return {
      message,
      pass,
    }
  },
  toMatchErrorMissingAuthHeader(errorResp: AxiosError<ErrorResponse>): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 401, 'ERR_MISSING_AUTH_HEADER')
  },
  toMatchErrorMalformedRequestSytantax(
    errorResp: AxiosError<ErrorResponse>,
  ): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 400, 'ERR_MALFORMED_REQUEST_SYNTAX')
  },
  toMatchErrorNotFound(errorResp: AxiosError<ErrorResponse>): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 404, 'ERR_NOT_FOUND')
  },
  toMatchErrorMethodNotAllowed(errorResp: AxiosError<ErrorResponse>): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 405, 'ERR_METHOD_NOT_ALLOWED')
  },
  toMatchErrorNotAcceptable(errorResp: AxiosError<ErrorResponse>): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 406, 'ERR_NOT_ACCEPTABLE')
  },
  toMatchErrorTokenInvalid(errorResp: AxiosError<ErrorResponse>): jest.CustomMatcherResult {
    return mkResultForEventService(errorResp, 400, 'ERR_TOKEN_INVALID')
  },
})
