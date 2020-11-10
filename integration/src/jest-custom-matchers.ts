/* eslint-disable @typescript-eslint/no-explicit-any */
expect.extend({
  myHelloMatcher(input: string): jest.CustomMatcherResult {
    return {
      message: () => 'hello world I am a custom matchert',
      pass: input === 'hello world' ? true : false,
    }
  },
  toMatchErrNodeUnreachable(response: any): jest.CustomMatcherResult {
    const errorCodeExpected = 'ERR_NODE_UNREACHABLE'
    const hasCodeError = response && 'code' in response && response.code === errorCodeExpected
    const hasMessage = response && 'message' in response
    const pass = hasCodeError && hasMessage

    const message = () =>
      `Expected code was: ${errorCodeExpected} instead got: ${response.code}, message was ${
        hasMessage ? 'found' : 'not found'
      }`
    return {
      message,
      pass,
    }
  },
})
