module.exports = {
  rootDir: '.',
  preset: 'ts-jest',
  globals: {
    'ts-jest': {
      tsConfig: 'tsconfig.json',
    },
  },
  testEnvironment: 'jsdom',
  moduleFileExtensions: ['js', 'json', 'jsx', 'ts', 'tsx'],
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
  setupFiles: ['<rootDir>/jest-setup.js'],
  moduleDirectories: ['node_modules', '<rootDir>'],
  maxWorkers: '50%',
}
