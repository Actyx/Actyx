import { none, Option, some } from 'fp-ts/lib/Option'
import { app, ipcRenderer } from 'electron'
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
} from '../../common/ipc'
import { isLeft } from 'fp-ts/lib/Either'
import { ioErrToStr } from '../../common/util'

export const getFolderFromUser = (): Promise<Option<string>> =>
  new Promise((resolve) => {
    const cleanUpIpcHandlers = () => {
      ipcRenderer.removeAllListeners(IpcToClient.FolderSelected)
      ipcRenderer.removeAllListeners(IpcToClient.FolderSelectedCancelled)
    }

    ipcRenderer.once(IpcToClient.FolderSelected, (event, args) => {
      const path = args[0]
      cleanUpIpcHandlers()
      resolve(some(path))
    })
    ipcRenderer.once(IpcToClient.FolderSelectedCancelled, (event, args) => {
      cleanUpIpcHandlers()
      resolve(none)
    })
    ipcRenderer.send(IpcFromClient.SelectFolder)
  })

export const getFileFromUser = (exts?: string[]): Promise<Option<string>> =>
  new Promise((resolve) => {
    const cleanUpIpcHandlers = () => {
      ipcRenderer.removeAllListeners(IpcToClient.FileSelected)
      ipcRenderer.removeAllListeners(IpcToClient.FileSelectedCancelled)
    }

    ipcRenderer.once(IpcToClient.FileSelected, (event, args) => {
      const path = args[0]
      cleanUpIpcHandlers()
      resolve(some(path))
    })
    ipcRenderer.once(IpcToClient.FileSelectedCancelled, (event, args) => {
      cleanUpIpcHandlers()
      resolve(none)
    })
    ipcRenderer.send(IpcFromClient.SelectFile, exts)
  })

export const getAppVersion = (): Promise<string> =>
  new Promise((resolve) => {
    ipcRenderer.once(IpcToClient.GotAppVersion, (_, version) => {
      resolve(version)
    })
    ipcRenderer.send(IpcFromClient.GetAppVersion)
  })

export const shutdownApp = () => {
  ipcRenderer.send(IpcFromClient.Shutdown)
}

export const toggleDevTools = () => {
  ipcRenderer.send(IpcFromClient.ToggleDevTools)
}

export const waitForFatalError = (): Promise<FatalError> =>
  new Promise((resolve) => {
    ipcRenderer.once(IpcToClient.FatalError, (_, arg) => {
      const error = arg as FatalError
      if (error.details) {
        console.log(error.details)
      }
      resolve(arg)
    })
  })

export const waitForNoUserKeysFound = (): Promise<void> =>
  new Promise((resolve) => {
    ipcRenderer.once(IpcToClient.NoUserKeysFound, () => {
      resolve()
    })
  })

const mkRpc =
  <Req, Resp>(rpc: RPC<Req, Resp>) =>
  (req: Req): Promise<Resp> =>
    new Promise((resolve, reject) => {
      ipcRenderer.once(rpc.ipcCode, (_, arg) => {
        if (isLeft(arg)) {
          console.log(`got error: ${JSON.stringify(arg.left)}`)
          reject(arg.left)
        } else {
          const resp = rpc.response.decode(arg.right)
          if (isLeft(resp)) {
            reject(`error decoding response for IPC RPC ${rpc.ipcCode}: ${ioErrToStr(resp.left)}`)
            return
          }
          resolve(resp.right)
        }
      })
      ipcRenderer.send(rpc.ipcCode, rpc.request.encode(req))
    })

export const getNodesDetails = mkRpc(RPC_GetNodesDetails)
export const setSettings = mkRpc(RPC_SetSettings)
export const shutdownNode = mkRpc(RPC_ShutdownNode)
export const createUserKeyPair = mkRpc(RPC_CreateUserKeyPair)
export const generateSwarmKey = mkRpc(RPC_GenerateSwarmKey)
export const signAppManifest = mkRpc(RPC_SignAppManifest)

export { Wizard, WizardFailure, WizardInput, WizardSuccess } from './wizard'

export const saveToClipboard = (str: string) => navigator.clipboard.writeText(str)
