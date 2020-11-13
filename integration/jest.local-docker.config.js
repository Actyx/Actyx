module.exports = {
  rootDir: '.',
  preset: 'ts-jest',
  globals: {
    'ts-jest': {
      tsConfig: 'tsconfig.json',
    },
  },
  moduleFileExtensions: ['js', 'json', 'jsx'],
  testPathIgnorePatterns: ['/node_modules/', '.+support\\.test\\.ts'],
  maxWorkers: '50%',
  setupFilesAfterEnv: ['./dist/src/jest-custom-matchers.js'],
  testTimeout: 120000,
  globalSetup: './dist/jest/setup-local-docker.js',
  globalTeardown: './dist/jest/teardown-local-docker.js',
  testMatch: ['<rootDir>/dist/src/local-docker/**/*.(test|spec).(js)'],
}
