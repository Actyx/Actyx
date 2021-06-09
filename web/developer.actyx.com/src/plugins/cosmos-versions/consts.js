// eslint-disable-next-line @typescript-eslint/no-var-requires
const path = require('path')
//const PRODUCTS = ['actyx', 'pond', 'cli', 'node-manager', 'ts-sdk', 'rust-sdk']
const PRODUCTS = ['actyx', 'cli', 'node-manager', 'pond']
const COSMOS_RELEASE_PATH =
  path.join(
    __dirname,
    '..',
    '..',
    '..',
    '..',
    '..',
    'rust',
    'release',
    'target',
    'release',
    'cosmos-release',
  ) + (process.platform === 'win32' ? '.exe' : '')
module.exports = {
  COSMOS_RELEASE_PATH,
  PRODUCTS,
}
