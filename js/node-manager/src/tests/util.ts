import { Application as App } from 'spectron'
import path from 'path'
import { default as fs, promises as fsPromises } from 'fs'
const appDataDir_ = require('app-data-folder')
let electronPath = path.join(__dirname, '..', '..', 'node_modules', '.bin', 'electron-forge')

if (process.platform === 'win32') {
  electronPath += '.cmd'
}

const getAppPathForCurrentPlatform = (platform: NodeJS.Platform): ReadonlyArray<string> => {
  switch (platform) {
    case 'win32': {
      return [
        __dirname,
        '..',
        '..',
        'out',
        'Actyx Node Manager-win32-x64',
        'actyx-node-manager.exe',
      ]
    }
    case 'darwin': {
      return ['/Applications/Actyx Node Manager.app/Contents/MacOS/actyx-node-manager']
    }
    default:
      throw 'Platform not suppoerted'
  }
}

const appPath = path.join(...getAppPathForCurrentPlatform(process.platform))

export const app = new App({
  path: appPath,
  env: {
    NODE_ENV: 'test',
  },
})

export const appDataDir = () => appDataDir_('actyx')

const keyDir = path.join(appDataDir(), 'keys', 'users')
const testPrivateKey = path.join(__dirname, 'e2e', 'fixtures', 'user-keys', 'testing')
const testPublicKey = path.join(__dirname, 'e2e', 'fixtures', 'user-keys', 'testing.pub')

export const removeTestKeys = async () => {
  if (!fs.existsSync(keyDir)) {
    return
  }
  await fsPromises.rmdir(keyDir, { recursive: true })
}

export const setTestKeys = async () => {
  await fsPromises.mkdir(keyDir, { recursive: true })
  await fsPromises.copyFile(testPrivateKey, path.join(keyDir, 'id'))
  await fsPromises.copyFile(testPublicKey, path.join(keyDir, 'id.pub'))
}
