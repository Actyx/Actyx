module.exports = {
  preset: 'ts-jest',
  collectCoverage: true,
  coverageDirectory: 'coverage',
  testTimeout: 100000,
  coverageThreshold: {
    global: {
      branches: 60,
      functions: 60,
      lines: 78,
      statements: 78,
    },
  },
  maxWorkers: '50%',
  testEnvironment: 'node',
  testPathIgnorePatterns: ['/node_modules/', '.+support.test.ts', '/dist/', '/native/', '/source/'],
}
