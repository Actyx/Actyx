import { none, Option, some } from "fp-ts/lib/Option";
import {
  FatalError,
  IpcFromClient,
  IpcToClient,
  RPC,
  RPC_SignAppManifest,
  RPC_CreateUserKeyPair,
  RPC_GenerateSwarmKey,
  RPC_ShutdownNode,
} from "../../common/ipc";
import { isLeft } from "fp-ts/lib/Either";
import { ioErrToStr } from "../../common/util";
import packageJson from "../../../package.json";
import {
  CreateUserKeyPairRequest,
  CreateUserKeyPairResponse,
  EventDiagnostic,
  GetNodeDetailsRequest,
  GetNodeDetailsResponse,
  GetNodesDetailsRequest,
  Node,
  NodeType,
  QueryRequest,
  QueryResponse,
  SetSettingsRequest,
  SetSettingsResponse,
} from "../../common/types";
import { ActyxAdminApi, create_private_key } from "ax-wasm";

export const getFolderFromUser = (): Promise<Option<string>> =>
  new Promise((resolve) => {
    resolve(none);
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
  });

export const getFileFromUser = (exts?: string[]): Promise<Option<string>> =>
  new Promise((resolve) => {
    resolve(none);
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
  });

export const getIsDev = (): Promise<boolean> =>
  new Promise((resolve) => {
    resolve(true);
    //   ipcRenderer.once(IpcToClient.GotIsDev, (_, isDev) => {
    //     resolve(isDev)
    //   })
    //   ipcRenderer.send(IpcFromClient.GetIsDev)
  });

export const shutdownApp = () => {
  // ipcRenderer.send(IpcFromClient.Shutdown)
};

export const toggleDevTools = () => {
  // ipcRenderer.send(IpcFromClient.ToggleDevTools)
};

export const waitForFatalError = (): Promise<FatalError> =>
  new Promise((resolve) => {
    // ipcRenderer.once(IpcToClient.FatalError, (_, arg) => {
    //   const error = arg as FatalError
    //   if (error.details) {
    //     console.log(error.details)
    //   }
    //   resolve(arg)
    // })
  });

const mkRpc =
  <Req, Resp>(rpc: RPC<Req, Resp>) =>
  (req: Req): Promise<Resp> =>
    Promise.reject();

//export const getNodesDetails = mkRpc(RPC_GetNodesDetails)
export const getNodesDetails = async (
  nodes: GetNodesDetailsRequest
): Promise<GetNodeDetailsResponse> => {
  if (nodes.length === 0) {
    return [];
  }
  try {
    return Promise.all(
      nodes.map(async ({ addr, api }) => {
        const details: Node = await api.get_node_details();
        return { ...details, addr };
      })
    );
  } catch (e) {
    console.error(e);
    return [];
  }
};
export const setSettings = async ({
  api,
  settings,
}: SetSettingsRequest): Promise<SetSettingsResponse> => {
  const response = await api.set_settings("com.actyx", settings);
  return response;
};
export const shutdownNode = mkRpc(RPC_ShutdownNode);
export const createUserKeyPair =
  async (): Promise<CreateUserKeyPairResponse> => {
    const privateKey = create_private_key();
    return {
      privateKey,
    };
  };
export const generateSwarmKey = mkRpc(RPC_GenerateSwarmKey);
export const signAppManifest = mkRpc(RPC_SignAppManifest);
export const query = ({
  query,
  api,
}: QueryRequest): AsyncIterable<EventDiagnostic[]> => {
  const buffer: EventDiagnostic[][] = [];
  let done = false;
  let err: Error | undefined = undefined;
  const pending: {
    res: (_: IteratorResult<EventDiagnostic[]>) => void;
    rej: (_: Error) => void;
  }[] = [];
  api.query_cb(query, (ev?: EventDiagnostic[], e?: Error) => {
    const p = pending.pop();
    if (p) {
      const { res, rej } = p;
      if (e) rej(e);
      // buffer is empty
      if (ev) {
        res({ done: false, value: ev });
      } else {
        res({ done: true, value: undefined });
      }
    } else {
      if (e) err = e;
      if (ev) {
        buffer.push(ev);
      } else {
        done = true;
      }
    }
  });
  return {
    [Symbol.asyncIterator]: () => ({
      next: (value?: any) => {
        if (err) return Promise.reject(err);
        else {
          if (done) {
            return Promise.resolve({ done, value: buffer.pop() });
          } else {
            const value = buffer.pop();
            if (value) {
              return Promise.resolve({ done, value });
            } else {
              return new Promise((res, rej) => {
                pending.push({ res, rej });
              });
            }
          }
        }
      },
    }),
  };
};

export { Wizard, WizardFailure, WizardInput, WizardSuccess } from "./wizard";

export const saveToClipboard = (str: string) =>
  navigator.clipboard.writeText(str);

export const getPackageVersion = (): string => packageJson.version;
