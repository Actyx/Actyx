console.log('running tests only for the event service')

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
  testRegex: ['/event-service/'],
  setupFilesAfterEnv: ['./dist/src/jest-custom-matchers.js'],
}
