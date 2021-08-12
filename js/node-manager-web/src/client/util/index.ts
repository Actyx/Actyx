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
  Node,
  NodeType,
  QueryRequest,
  QueryResponse,
  SetSettingsRequest,
  SetSettingsResponse,
} from '../../common/types'
import { ActyxAdminApi, create_private_key } from 'ax-wasm'

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

const mkRpc =
  <Req, Resp>(rpc: RPC<Req, Resp>) =>
  (req: Req): Promise<Resp> =>
    new Promise((resolve, reject) => {})

//export const getNodesDetails = mkRpc(RPC_GetNodesDetails)
export const getNodesDetails = async (
  nodes: GetNodesDetailsRequest,
): Promise<GetNodeDetailsResponse> => {
  if (nodes.length === 0) {
    return []
  }
  try {
    return Promise.all(
      nodes.map(async ({ addr, privateKey }) => {
        const api = new ActyxAdminApi(addr, privateKey)

        const details: Node = await api.get_node_details()
        return { ...details, addr }
      }),
    )
  } catch (e) {
    console.error(e)
    return []
  }
}
export const setSettings = async ({
  addr,
  privateKey,
  settings,
}: SetSettingsRequest): Promise<SetSettingsResponse> => {
  const api = new ActyxAdminApi(addr, privateKey)
  const response = await api.set_settings('com.actyx', settings)
  return response
}
export const shutdownNode = mkRpc(RPC_ShutdownNode)
export const createUserKeyPair = async (): Promise<CreateUserKeyPairResponse> => {
  const privateKey = create_private_key()
  return {
    privateKey,
  }
}
export const generateSwarmKey = mkRpc(RPC_GenerateSwarmKey)
export const signAppManifest = mkRpc(RPC_SignAppManifest)
export const query = async ({ addr, query, privateKey }: QueryRequest): Promise<QueryResponse> => {
  const api = new ActyxAdminApi(addr, privateKey)
  const response = await api.query(query)
  return response
}

export { Wizard, WizardFailure, WizardInput, WizardSuccess } from './wizard'

export const saveToClipboard = (str: string) => navigator.clipboard.writeText(str)

export const getPackageVersion = (): string => packageJson.version
