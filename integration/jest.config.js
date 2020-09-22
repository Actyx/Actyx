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
  testEnvironment: './dist/jest/environment.js',
  moduleFileExtensions: ['js', 'json', 'jsx'],
  testPathIgnorePatterns: ['/node_modules/', '.+support\\.test\\.ts'],
  maxWorkers: '50%',
}
