import { none, Option, some } from 'fp-ts/lib/Option'
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
} from '../../common/ipc'
import { isLeft } from 'fp-ts/lib/Either'
import { ioErrToStr } from '../../common/util'
import packageJson from '../../../package.json'
import {
  CreateUserKeyPairRequest,
  CreateUserKeyPairResponse,
  GetNodeDetailsRequest,
  GetNodeDetailsResponse,
  GetNodesDetailsRequest,
  NodeType,
  QueryRequest,
  QueryResponse,
} from '../../common/types'
import { ActyxAdminApi } from 'ax-wasm'

const api = new ActyxAdminApi('0oym7jtFXHERwneFUMOzzgInvJEjk9ZOVhe0AHpMDOuU=')
export const getFolderFromUser = (): Promise<Option<string>> =>
  new Promise((resolve) => {
    resolve(none)
    // const cleanUpIpcHandlers = () => {
    //   ipcRenderer.removeAllListeners(IpcToClient.FolderSelected)
    //   ipcRenderer.removeAllListeners(IpcToClient.FolderSelectedCancelled)
    // }

    // ipcRenderer.once(IpcToClient.FolderSelected, (event, args) => {
    //   const path = args[0]
    //   cleanUpIpcHandlers()
    //   resolve(some(path))
    // })
    // ipcRenderer.once(IpcToClient.FolderSelectedCancelled, (event, args) => {
    //   cleanUpIpcHandlers()
    //   resolve(none)
    // })
    // ipcRenderer.send(IpcFromClient.SelectFolder)
  })

export const getFileFromUser = (exts?: string[]): Promise<Option<string>> =>
  new Promise((resolve) => {
    resolve(none)
    // const cleanUpIpcHandlers = () => {
    //   ipcRenderer.removeAllListeners(IpcToClient.FileSelected)
    //   ipcRenderer.removeAllListeners(IpcToClient.FileSelectedCancelled)
    // }

    // ipcRenderer.once(IpcToClient.FileSelected, (event, args) => {
    //   const path = args[0]
    //   cleanUpIpcHandlers()
    //   resolve(some(path))
    // })
    // ipcRenderer.once(IpcToClient.FileSelectedCancelled, (event, args) => {
    //   cleanUpIpcHandlers()
    //   resolve(none)
    // })
    // ipcRenderer.send(IpcFromClient.SelectFile, exts)
  })

export const getIsDev = (): Promise<boolean> =>
  new Promise((resolve) => {
    resolve(true)
    //   ipcRenderer.once(IpcToClient.GotIsDev, (_, isDev) => {
    //     resolve(isDev)
    //   })
    //   ipcRenderer.send(IpcFromClient.GetIsDev)
  })

export const shutdownApp = () => {
  // ipcRenderer.send(IpcFromClient.Shutdown)
}

export const toggleDevTools = () => {
  // ipcRenderer.send(IpcFromClient.ToggleDevTools)
}

export const waitForFatalError = (): Promise<FatalError> =>
  new Promise((resolve) => {
    // ipcRenderer.once(IpcToClient.FatalError, (_, arg) => {
    //   const error = arg as FatalError
    //   if (error.details) {
    //     console.log(error.details)
    //   }
    //   resolve(arg)
    // })
  })

export const waitForNoUserKeysFound = (): Promise<void> =>
  new Promise((resolve) => {
    resolve()
    // ipcRenderer.once(IpcToClient.NoUserKeysFound, () => {
    //   resolve()
    // })
  })

const mkRpc =
  <Req, Resp>(rpc: RPC<Req, Resp>) =>
  (req: Req): Promise<Resp> =>
    new Promise((resolve, reject) => {
      // ipcRenderer.once(rpc.ipcCode, (_, arg) => {
      //   if (isLeft(arg)) {
      //     console.log(`got error: ${JSON.stringify(arg.left)}`)
      //     reject(arg.left)
      //   } else {
      //     const resp = rpc.response.decode(arg.right)
      //     if (isLeft(resp)) {
      //       reject(`error decoding response for IPC RPC ${rpc.ipcCode}: ${ioErrToStr(resp.left)}`)
      //       return
      //     }
      //     resolve(resp.right)
      //   }
      // })
      // ipcRenderer.send(rpc.ipcCode, rpc.request.encode(req))
    })

//export const getNodesDetails = mkRpc(RPC_GetNodesDetails)
export const getNodesDetails = async ({
  addrs,
}: GetNodesDetailsRequest): Promise<GetNodeDetailsResponse> => {
  if (addrs.length === 0) {
    return []
  }
  try {
    // FIXME
    const details = await api.get_node_details()
    const ret = [{ ...details, addr: addrs[0] }]
    return ret
  } catch (e) {
    console.error(e)
    return []
  }
}
export const setSettings = mkRpc(RPC_SetSettings)
export const shutdownNode = mkRpc(RPC_ShutdownNode)
// export const createUserKeyPair = mkRpc(RPC_CreateUserKeyPair)
export const createUserKeyPair = async (
  req: CreateUserKeyPairRequest,
): Promise<CreateUserKeyPairResponse> => {
  return {
    privateKeyPath: 'FIXME',
    publicKey: 'FIXME',
    publicKeyPath: 'FIXME',
  }
}
export const generateSwarmKey = mkRpc(RPC_GenerateSwarmKey)
export const signAppManifest = mkRpc(RPC_SignAppManifest)
export const query = async (req: QueryRequest): Promise<QueryResponse> => {
  const response = await api.query(req.query)
  return response
}

export { Wizard, WizardFailure, WizardInput, WizardSuccess } from './wizard'

export const saveToClipboard = (str: string) => navigator.clipboard.writeText(str)

export const getPackageVersion = (): string => packageJson.version
