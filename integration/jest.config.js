// This is visible in globalSetup, globalTeardown, and via globals below also in tests.
// The idea is that its contents are provided in globalSetup.
global.nodeSetup = {}

module.exports = {
  rootDir: '.',
  preset: 'ts-jest',
  globals: {
    'ts-jest': {
      tsConfig: 'tsconfig.json',
    },
    nodeSetup: global.nodeSetup,
  },
  globalSetup: './dist/jest/setup.js',
  globalTeardown: './dist/jest/teardown.js',
  testEnvironment: 'node',
  moduleFileExtensions: ['js', 'json', 'jsx'],
  collectCoverage: true,
  collectCoverageFrom: ['**/*.{ts,tsx}', '!**/*.{stories.tsx}', '!**/*.d.ts'],
  coveragePathIgnorePatterns: ['/node_modules/', 'src/hosts.ts'],
  coverageDirectory: '<rootDir>/coverage',
  coverageThreshold: {
    global: {
      statements: 75,
      branches: 53,
      lines: 76,
      functions: 56,
    },
  },
  testPathIgnorePatterns: ['/node_modules/', '.+support\\.test\\.ts'],
  maxWorkers: '50%',
}
