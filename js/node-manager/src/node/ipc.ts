import { StoreData } from '../common/types'
import { ipcMain, dialog, BrowserWindow, App } from 'electron'
import {
  FatalError,
  IpcFromClient,
  IpcToClient,
  RPC,
  RPC_SignAppManifest,
  RPC_CreateUserKeyPair,
  RPC_GenerateSwarmKey,
  RPC_GetNodesDetails,
  RPC_SetSettings,
  RPC_ShutdownNode,
  RPC_Query,
} from '../common/ipc'
import { readStore, writeStore, storePath } from './store'
import {
  createUserKeyPair,
  generateSwarmKey,
  getNodesDetails,
  setSettings,
  signAppManifest,
  shutdownNode,
  query,
} from './tasks'
import { isLeft, left, right } from 'fp-ts/lib/Either'
import { ioErrToStr, safeErrorToStr } from '../common/util'
import { FileFilter } from 'electron/main'
import { isDev } from './util'

export const triggerFatalError = (browserWindow: BrowserWindow, error: FatalError) => {
  console.log(`triggering fatal error: ${JSON.stringify(error)}`)
  browserWindow.webContents.send(IpcToClient.FatalError, error)
}

export const triggerNoUserKeysFound = (browserWindow: BrowserWindow) => {
  console.log(`triggering no user keys found`)
  browserWindow.webContents.send(IpcToClient.NoUserKeysFound)
}

const setupRpc = <Req, Resp>(
  window: BrowserWindow,
  rpc: RPC<Req, Resp>,
  action: (req: Req) => Promise<Resp>,
): void => {
  ipcMain.on(rpc.ipcCode, async (event, arg) => {
    const req = rpc.request.decode(arg)
    if (isLeft(req)) {
      triggerFatalError(window, {
        shortMessage: `IPC RPC decoding error; unable to decode argument for request ${rpc.ipcCode}`,
        details: ioErrToStr(req.left),
      })
      return
    }
    try {
      const resp = await action(req.right)
      event.reply(rpc.ipcCode, right(rpc.response.encode(resp)))
    } catch (error) {
      // This is a bit of a hack
      const safeError = safeErrorToStr(error)
      console.log(`safeError:`)
      console.log(safeError)
      if (
        safeError.includes('ERR_USER_UNAUTHENTICATED') &&
        safeError.includes('Unable to authenticate with node since no user keys found in')
      ) {
        triggerNoUserKeysFound(window)
      } else {
        const err: FatalError = {
          shortMessage: safeError,
        }
        event.reply(rpc.ipcCode, left(err))
      }
      return
    }
  })
}

export const setupIpc = (app: App, browserWindow: BrowserWindow) => {
  setupRpc(browserWindow, RPC_GetNodesDetails, getNodesDetails)
  setupRpc(browserWindow, RPC_SetSettings, setSettings)
  setupRpc(browserWindow, RPC_CreateUserKeyPair, createUserKeyPair)
  setupRpc(browserWindow, RPC_GenerateSwarmKey, generateSwarmKey)
  setupRpc(browserWindow, RPC_SignAppManifest, signAppManifest)
  setupRpc(browserWindow, RPC_ShutdownNode, shutdownNode)
  setupRpc(browserWindow, RPC_Query, query)

  ipcMain.on(IpcFromClient.SelectFolder, async (event, arg) => {
    console.log(`[ipc] got request ${IpcFromClient.SelectFolder}`)
    const res = await dialog.showOpenDialog(browserWindow, {
      properties: ['openDirectory', 'createDirectory'],
    })
    if (res.canceled) {
      console.log(`[ipc] sending response ${IpcToClient.FolderSelectedCancelled}`)
      event.reply(IpcToClient.FolderSelectedCancelled)
    } else {
      console.log(`[ipc] sending response ${IpcToClient.FolderSelected}`)
      event.reply(IpcToClient.FolderSelected, res.filePaths)
    }
  })
  ipcMain.on(IpcFromClient.SelectFile, async (event, exts: string[]) => {
    console.log(`[ipc] got request ${IpcFromClient.SelectFile}`)
    const ff: FileFilter | undefined = !exts
      ? undefined
      : {
          extensions: exts,
          name: '',
        }

    const res = await dialog.showOpenDialog(browserWindow, {
      properties: ['openFile'],
      filters: ff ? [ff] : undefined,
    })
    if (res.canceled) {
      console.log(`[ipc] sending response ${IpcToClient.FileSelectedCancelled}`)
      event.reply(IpcToClient.FileSelectedCancelled)
    } else {
      console.log(`[ipc] sending response ${IpcToClient.FileSelected}`)
      event.reply(IpcToClient.FileSelected, res.filePaths)
    }
  })
  ipcMain.on(IpcFromClient.Shutdown, async () => {
    console.log(`[ipc] got request ${IpcFromClient.Shutdown}`)
    app.exit()
  })
  ipcMain.on(IpcFromClient.ToggleDevTools, async () => {
    console.log(`[ipc] got request ${IpcFromClient.Shutdown}`)
    if (browserWindow.webContents.isDevToolsOpened()) {
      browserWindow.webContents.closeDevTools()
    } else {
      browserWindow.webContents.openDevTools()
    }
  })
  ipcMain.on(IpcFromClient.GetIsDev, async (event) => {
    event.reply(IpcToClient.GotIsDev, isDev())
  })

  ipcMain.on(IpcFromClient.LoadStore, async (event, new_data: null | StoreData) => {
    console.log(`[ipc] got request ${IpcFromClient.LoadStore}`)
    // Must save data
    if (new_data !== null) {
      try {
        await writeStore(app, new_data)
      } catch (error) {
        triggerFatalError(browserWindow, {
          shortMessage: `error writing to store at ${storePath(app)}`,
          details: safeErrorToStr(error),
        })
      }
    }

    // Load data
    try {
      event.reply(IpcToClient.StoreLoaded, await readStore(app))
    } catch (error) {
      triggerFatalError(browserWindow, {
        shortMessage: `error reading store from ${storePath(app)}`,
        details: safeErrorToStr(error),
      })
    }
  })
}
