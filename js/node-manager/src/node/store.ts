import { App } from 'electron'
import path from 'path'
import { StoreData } from '../common/types'
import { promises as fs, existsSync } from 'fs'
import { isDev } from '../node/util'

const INITIAL_STORE_DATA: StoreData = {
  preferences: {
    favoriteNodeAddrs: [],
  },
}

export const storePath = (app: App) => {
  const userDataDir = isDev() ? '.' : app.getPath('userData')
  const storeFileName = 'store.json'
  return path.join(userDataDir, storeFileName)
}

const setupStore = async (app: App) => await writeStore(app, INITIAL_STORE_DATA)
export const setupStoreIfDoesntExist = async (app: App) => {
  if (!existsSync(storePath(app))) {
    setupStore(app)
  }
}

export const writeStore = async (app: App, data: StoreData): Promise<void> =>
  await fs.writeFile(storePath(app), JSON.stringify(data, null, 2))

export const readStore = async (app: App): Promise<StoreData> => {
  setupStoreIfDoesntExist(app)
  return JSON.parse(await fs.readFile(storePath(app), 'utf-8')) as StoreData
}
