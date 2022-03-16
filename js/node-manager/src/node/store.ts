import { App } from 'electron'
import path from 'path'
import { StoreData } from '../common/types'
import { promises as fs, existsSync } from 'fs'
import { isDev } from '../node/util'
import { v4 as uuidv4 } from 'uuid'
import { isLeft } from 'fp-ts/lib/Either'
import { formatValidationErrors } from 'io-ts-reporters'
import { sleep } from '../common/util'

const INITIAL_STORE_DATA: StoreData = {
  preferences: {
    favoriteNodeAddrs: [],
    nodeTimeout: undefined,
  },
  analytics: {
    disabled: false,
    userId: uuidv4(),
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
    await setupStore(app)
  }
}

export const deleteStore = async (app: App) => {
  await fs.unlink(storePath(app))
}

export const writeStore = async (app: App, data: StoreData): Promise<void> =>
  await fs.writeFile(storePath(app), JSON.stringify(data, null, 2))

export const readStore = async (app: App): Promise<StoreData> => {
  await setupStoreIfDoesntExist(app)

  try {
    const json = JSON.parse(await fs.readFile(storePath(app), 'utf-8'))
    const data = StoreData.decode(json)
    if (isLeft(data)) {
      throw new Error(`error decoding store: ${formatValidationErrors(data.left)}`)
    }

    return data.right
  } catch (error) {
    console.error(`error loading store: ${error}`)
    try {
      await deleteStore(app)
      return await readStore(app)
    } catch (delError) {
      const msg = `fatal error: unable to recover from store error (${error}) by deleting: ${delError}`
      console.error(msg)
      throw new Error(msg)
    }
  }
}
