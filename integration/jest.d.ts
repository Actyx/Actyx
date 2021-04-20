export {}
declare global {
  namespace jest {
    interface Matchers<R> {
      toMatchErrNodeUnreachable(): CustomMatcherResult
      toMatchErrInvalidInput(): CustomMatcherResult
      toMatchCodeOk(): CustomMatcherResult
      toMatchErrorMissingAuthHeader(): CustomMatcherResult
      toMatchErrorMalformedRequestSytantax(): CustomMatcherResult
      toMatchErrorNotFound(): CustomMatcherResult
      toMatchErrorMethodNotAllowed(): CustomMatcherResult
      toMatchErrorNotAcceptable(): CustomMatcherResult
      toMatchErrorTokenInvalid(): CustomMatcherResult
    }
  }
}
