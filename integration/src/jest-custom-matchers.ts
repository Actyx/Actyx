expect.extend({
  myHelloMatcher(input: string): jest.CustomMatcherResult {
    return {
      message: () => 'hello world I am a custom matchert',
      pass: input === 'hello world' ? true : false,
    }
  },
})
