import React, { useContext, useEffect, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  getAppVersion,
  getNodesDetails,
  setSettings,
  waitForNoUserKeysFound,
  shutdownNode,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  Node,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'

const POLLING_INTERVAL_MS = 1000

export const reducer = (state: AppState, action: AppAction): AppState => {
  switch (action.key) {
    case AppActionKey.ShowOverview: {
      return { ...state, key: AppStateKey.Overview }
    }
    case AppActionKey.ShowSetupUserKey: {
      return { ...state, key: AppStateKey.SetupUserKey }
    }
    case AppActionKey.ShowAbout: {
      return { ...state, key: AppStateKey.About }
    }
    case AppActionKey.ShowAppSigning: {
      return { ...state, key: AppStateKey.AppSigning }
    }
    case AppActionKey.ShowNodeAuth: {
      return { ...state, key: AppStateKey.NodeAuth }
    }
    case AppActionKey.ShowDiagnostics: {
      return { ...state, key: AppStateKey.Diagnostics }
    }
    case AppActionKey.ShowNodeDetail: {
      return { ...state, ...action, key: AppStateKey.NodeDetail }
    }
    case AppActionKey.ShowGenerateSwarmKey: {
      return { ...state, ...action, key: AppStateKey.SwarmKey }
    }
    case AppActionKey.SetVersion: {
      return { ...state, version: action.version }
    }
  }
}

interface Data {
  nodes: Node[]
}

interface Actions {
  addNodes: (addrs: string[]) => void
  remNodes: (addrs: string[]) => void
  setSettings: (addr: string, settings: object) => Promise<void>
  shutdownNode: (addr: string) => Promise<void>
  createUserKeyPair: (privateKeyPath: string | null) => Promise<CreateUserKeyPairResponse>
  generateSwarmKey: () => Promise<GenerateSwarmKeyResponse>
  signAppManifest: ({
    pathToManifest,
    pathToCertificate,
  }: {
    pathToManifest: string
    pathToCertificate: string
  }) => Promise<SignAppManifestResponse>
}

export type AppDispatch = (action: AppAction) => void
const AppStateContext =
  React.createContext<
    | {
        state: AppState
        data: Data
        actions: Actions
        dispatch: AppDispatch
      }
    | undefined
  >(undefined)

export const AppStateProvider: React.FC = ({ children }) => {
  const [state, dispatch] = useReducer(reducer, { key: AppStateKey.Overview, version: '' })
  const [data, setData] = useState<Data>({ nodes: [] })

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => ({
        ...current,
        nodes: current.nodes.concat(
          addrs.map((addr) => ({
            type: NodeType.Loading,
            addr,
          })),
        ),
      }))
    },
    remNodes: (addrs) => {
      setData((current) => ({
        ...current,
        nodes: current.nodes.filter((n) => !addrs.includes(n.addr)),
      }))
    },
    setSettings: (addr, settings) => setSettings({ addr, settings }),
    shutdownNode: (addr) => shutdownNode({ addr }),
    createUserKeyPair: (privateKeyPath) => createUserKeyPair({ privateKeyPath }),
    generateSwarmKey: () => generateSwarmKey({}),
    signAppManifest: ({ pathToManifest, pathToCertificate }) =>
      signAppManifest({
        pathToManifest,
        pathToCertificate,
      }),
  }

  useEffect(() => {
    ;(async () => {
      await waitForNoUserKeysFound()
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    })()
  }, [])

  useEffect(() => {
    ;(async () => {
      const version = await getAppVersion()
      dispatch({ key: AppActionKey.SetVersion, version })
    })()
  }, [])

  useEffect(() => {
    let unmounted = false

    let timeout: ReturnType<typeof setTimeout> | null = null
    const getDetailsAndUpdate = async () => {
      const nodes = await getNodesDetails({ addrs: data.nodes.map((n) => n.addr) })
      if (!unmounted) {
        setData((current) => ({
          ...current,
          nodes: current.nodes
            .filter((n) => !nodes.map((n) => n.addr).includes(n.addr))
            .concat(nodes),
        }))
        timeout = setTimeout(() => {
          getDetailsAndUpdate()
        }, POLLING_INTERVAL_MS)
      }
    }

    if (state.key !== 'SetupUserKey') {
      timeout = setTimeout(getDetailsAndUpdate, POLLING_INTERVAL_MS)
    }

    return () => {
      unmounted = true
      if (timeout !== null) {
        clearTimeout(timeout)
      }
    }
  }, [data.nodes, state.key])

  return (
    <AppStateContext.Provider value={{ state, data, actions, dispatch }}>
      {children}
    </AppStateContext.Provider>
  )
}

export const useAppState = () => {
  const c = useContext(AppStateContext)
  if (c === undefined) {
    throw 'AppStateContext is undefined'
  }
  return c
}
