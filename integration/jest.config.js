// This is visible in globalSetup, globalTeardown, and via globals below also in tests.
// The idea is that its contents are provided in globalSetup.
global.axNodeSetup = {}

// Reference documentation: https://jestjs.io/docs/en/configuration
module.exports = {
  rootDir: '.',
  preset: 'ts-jest',
  // A set of global variables that need to be available in all test
  // environments. [..] Note that, if you specify a global reference value (like
  // an object or array) here, and some code mutates that value in the midst of
  // running a test, that mutation will not be persisted across test runs for
  // other test files. In addition, the globals object must be
  // json-serializable, so it can't be used to specify global functions.
  globals: {
    'ts-jest': {
      tsConfig: 'tsconfig.json',
    },
    axNodeSetup: global.axNodeSetup,
  },
  moduleFileExtensions: ['js', 'json', 'jsx', 'ts', 'tsx'],
  testPathIgnorePatterns: ['/node_modules/', '.+support\\.test\\.ts'],
  maxWorkers: '50%',
  // A list of paths to modules that run some code to configure or set up the
  // testing framework before each test file in the suite is executed. Since
  // setupFiles executes before the test framework is installed in the
  // environment, this script file presents you the opportunity of running some
  // code immediately after the test framework has been installed in the
  // environment.
  setupFilesAfterEnv: ['./src/jest-custom-matchers.ts'],
  testTimeout: 120000,
  // This option allows the use of a custom global setup module which exports an
  // async function that is triggered once before all test suites. This function
  // gets Jest's globalConfig object as a parameter.
  globalSetup: './jest/setup.ts',
  //  This option allows the use of a custom global teardown module which exports
  //  an async function that is triggered once after all test suites. This function
  //  gets Jest's globalConfig object as a parameter.
  globalTeardown: './jest/teardown.ts',
  // The test environment that will be used for testing. [..] You can create
  // your own module that will be used for setting up the test environment. The
  // module must export a class with setup, teardown and runScript methods. You
  // can also pass variables from this module to your test suites by assigning
  // them to this.global object – this will make them available in your test
  // suites as global variables.
  testEnvironment: './jest/environment.ts',
}
