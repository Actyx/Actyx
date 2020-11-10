export {}
declare global {
  namespace jest {
    interface Matchers<R> {
      myHelloMatcher(): CustomMatcherResult
      toMatchErrNodeUnreachable(): CustomMatcherResult
      toMatchErrInvalidInput(): CustomMatcherResult
      toMatchCodeOk(): CustomMatcherResult
    }
  }
}
