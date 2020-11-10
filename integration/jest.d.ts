export {}
declare global {
  namespace jest {
    interface Matchers<R> {
      toMatchErrNodeUnreachable(): CustomMatcherResult
      toMatchErrInvalidInput(): CustomMatcherResult
      toMatchCodeOk(): CustomMatcherResult
    }
  }
}
