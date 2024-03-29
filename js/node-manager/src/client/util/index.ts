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
  RPC_GetNodeDetails,
  RPC_SetSettings,
  RPC_ShutdownNode,
  RPC_Query,
  RPC_Connect,
  RPC_TopicLs,
  RPC_TopicDelete,
  RPC_Publish,
} from '../../common/ipc'
import { isLeft } from 'fp-ts/lib/Either'
import { ioErrToStr } from '../../common/util'
import packageJson from '../../../package.json'

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

export const getIsDev = (): Promise<boolean> =>
  new Promise((resolve) => {
    ipcRenderer.once(IpcToClient.GotIsDev, (_, isDev) => {
      resolve(isDev)
    })
    ipcRenderer.send(IpcFromClient.GetIsDev)
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
    ipcRenderer.invoke(rpc.ipcCode, rpc.request.encode(req)).then((arg) => {
      if (isLeft(arg)) {
        console.log(`got error: ${JSON.stringify(arg.left)}`)
        throw arg.left
      } else {
        const resp = rpc.response.decode(arg.right)
        if (isLeft(resp)) {
          throw new Error(
            `error decoding response for IPC RPC ${rpc.ipcCode}: ${ioErrToStr(resp.left)}`,
          )
        }
        return resp.right
      }
    })

export const connect = mkRpc(RPC_Connect)
export const getNodeDetails = mkRpc(RPC_GetNodeDetails)
export const setSettings = mkRpc(RPC_SetSettings)
export const shutdownNode = mkRpc(RPC_ShutdownNode)
export const createUserKeyPair = mkRpc(RPC_CreateUserKeyPair)
export const generateSwarmKey = mkRpc(RPC_GenerateSwarmKey)
export const signAppManifest = mkRpc(RPC_SignAppManifest)
export const query = mkRpc(RPC_Query)
export const publish = mkRpc(RPC_Publish)
export const getTopicList = mkRpc(RPC_TopicLs)
export const deleteTopic = mkRpc(RPC_TopicDelete)

export { Wizard, WizardFailure, WizardInput, WizardSuccess } from './wizard'

export const saveToClipboard = (str: string) => navigator.clipboard.writeText(str)

export const getPackageVersion = (): string => packageJson.version
