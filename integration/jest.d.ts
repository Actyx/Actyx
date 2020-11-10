export {}
declare global {
  namespace jest {
    interface Matchers<R> {
      myHelloMatcher(): CustomMatcherResult
    }
  }
}
