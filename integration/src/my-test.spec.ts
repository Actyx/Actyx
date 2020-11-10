import '../jest-custom-matchers'

describe('myMatcher', () => {
  it('should false true if hello world is passed', () => {
    expect('hello world').myHelloMatcher()
  })
  it('should not pass if not hello world', () => {
    expect('foo').myHelloMatcher()
  })
})
