import React, { useContext, useEffect, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  getNodesDetails,
  setSettings,
  shutdownNode,
  query,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  Node,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
  QueryResponse,
  SwarmOffsets,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
import { useAnalytics } from '../analytics'
import { AnalyticsActions } from '../analytics/types'
import { FatalError } from '../../common/ipc'
import { safeErrorToStr } from '../../common/util'
import deepEqual from 'fast-deep-equal'
import { OffsetInfo } from '../offsets'
import { none, Option, some, fromNullable, map } from 'fp-ts/lib/Option'
import { useStore } from '../store'
import { store } from 'fp-ts/lib/Store'
import { StringType } from 'io-ts'

const POLLING_INTERVAL_MS = 1_000

export const reducer =
  (analytics: AnalyticsActions | undefined) =>
  (state: AppState, action: AppAction): AppState => {
    switch (action.key) {
      case AppActionKey.ShowOverview: {
        if (analytics) {
          analytics.viewedScreen('Overview')
        }
        return { ...state, key: AppStateKey.Overview }
      }
      case AppActionKey.ShowSetupUserKey: {
        if (analytics) {
          analytics.viewedScreen('SetupUserKey')
        }
        return { ...state, key: AppStateKey.SetupUserKey }
      }
      case AppActionKey.ShowAbout: {
        if (analytics) {
          analytics.viewedScreen('About')
        }
        return { ...state, key: AppStateKey.About }
      }
      case AppActionKey.ShowAppSigning: {
        if (analytics) {
          analytics.viewedScreen('AppSigning')
        }
        return { ...state, key: AppStateKey.AppSigning }
      }
      case AppActionKey.ShowNodeAuth: {
        if (analytics) {
          analytics.viewedScreen('NodeAuth')
        }
        return { ...state, key: AppStateKey.NodeAuth }
      }
      case AppActionKey.ShowDiagnostics: {
        if (analytics) {
          analytics.viewedScreen('Diagnostics')
        }
        return { ...state, key: AppStateKey.Diagnostics }
      }
      case AppActionKey.ShowNodeDetail: {
        if (analytics) {
          analytics.viewedScreen('NodeDetail')
        }
        return { ...state, ...action, key: AppStateKey.NodeDetail }
      }
      case AppActionKey.ShowGenerateSwarmKey: {
        if (analytics) {
          analytics.viewedScreen('GenerateSwarmKey')
        }
        return { ...state, ...action, key: AppStateKey.SwarmKey }
      }
      case AppActionKey.ShowPreferences: {
        if (analytics) {
          analytics.viewedScreen('Preferences')
        }
        return { ...state, ...action, key: AppStateKey.Preferences }
      }
      case AppActionKey.ShowQuery: {
        if (analytics) {
          analytics.viewedScreen('Query')
        }
        return { ...state, ...action, key: AppStateKey.Query }
      }
    }
  }

interface Data {
  nodes: Node[]
  offsets: Option<OffsetInfo>
  privateKey: Option<string>
}

interface Actions {
  addNodes: (addrs: string[]) => void
  remNodes: (addrs: string[]) => void
  setSettings: (addr: string, privateKey: string, settings: object) => Promise<void>
  shutdownNode: (addr: string, privateKey: string) => Promise<void>
  createUserKeyPair: () => Promise<CreateUserKeyPairResponse>
  generateSwarmKey: () => Promise<GenerateSwarmKeyResponse>
  signAppManifest: ({
    pathToManifest,
    pathToCertificate,
  }: {
    pathToManifest: string
    pathToCertificate: string
  }) => Promise<SignAppManifestResponse>
  query: (args: { addr: string; privateKey: string; query: string }) => Promise<QueryResponse>
}

export type AppDispatch = (action: AppAction) => void
const AppStateContext = React.createContext<
  | {
      state: AppState
      data: Data
      actions: Actions
      dispatch: AppDispatch
    }
  | undefined
>(undefined)

export const AppStateProvider: React.FC<{
  setFatalError: (error: FatalError) => void
}> = ({ children, setFatalError }) => {
  const analytics = useAnalytics()
  const [state, dispatch] = useReducer(reducer(analytics), {
    key: AppStateKey.Overview,
  })
  const [data, setData] = useState<Data>({ nodes: [], offsets: none, privateKey: none })

  const store = useStore()
  const privateKey: string | undefined = store.key === 'Loaded' ? store.data.privateKey : undefined

  console.log('priv', privateKey)
  useEffect(() => {
    setData((current) => ({ ...current, privateKey: fromNullable(privateKey) }))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [privateKey])

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => {
        if (analytics) {
          addrs.forEach((addr) => {
            if (current.nodes.find((node) => node.addr !== addr)) {
              analytics.addedNode()
            }
          })
        }
        return {
          ...current,
          nodes: current.nodes.concat(
            addrs.map((addr) => ({
              type: NodeType.Loading,
              addr,
              offsets: null,
            })),
          ),
        }
      })
    },
    remNodes: (addrs) => {
      if (analytics) {
        addrs.forEach(() => {
          analytics.removedNode()
        })
      }
      setData((current) => ({
        ...current,
        nodes: current.nodes.filter((n) => !addrs.includes(n.addr)),
      }))
    },
    setSettings: (addr, settings) => {
      if (analytics) {
        analytics.setSettings()
      }
      return setSettings({ addr, settings, privateKey: privateKey! })
    },
    shutdownNode: (addr) => {
      if (analytics) {
        analytics.shutdownNode()
      }
      return shutdownNode({ addr, privateKey: privateKey! })
    },
    createUserKeyPair: async () => {
      if (analytics) {
        analytics.createdUserKeyPair()
      }
      const key = await createUserKeyPair()
      if (store.key === 'Loaded') {
        store.actions.updateAndReload({
          ...store.data,
          privateKey: key.privateKey,
        })
      }
      return key
    },
    generateSwarmKey: () => {
      if (analytics) {
        analytics.generatedSwarmKey()
      }
      return generateSwarmKey({})
    },
    signAppManifest: ({ pathToManifest, pathToCertificate }) => {
      if (analytics) {
        analytics.signedAppManifest()
      }
      return signAppManifest({
        pathToManifest,
        pathToCertificate,
      })
    },
    query: (args) => {
      if (analytics) {
        analytics.queriedEvents(args.query)
      }
      return query(args)
    },
  }

  useEffect(() => {
    if (store.key === 'Loaded' && !store.data.privateKey) {
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store.key])

  useEffect(() => {
    let unmounted = false

    let timeout: ReturnType<typeof setTimeout> | null = null
    const getDetailsAndUpdate = async () => {
      console.log('getting node information')
      try {
        const nodes = await getNodesDetails(
          data.nodes.map((n) => ({ addr: n.addr, privateKey: privateKey! })),
        )
        const offsetsInfo = OffsetInfo.of(data.nodes)
        if (!unmounted) {
          if (!deepEqual(data.nodes, nodes) || !deepEqual(data.offsets, some(offsetsInfo))) {
            console.log(`+++ updating app-state/nodes +++`)
            setData({
              ...data,
              offsets: some(offsetsInfo),
              nodes: data.nodes
                .filter((n) => !nodes.map((n) => n.addr).includes(n.addr))
                .concat(nodes.filter((n) => data.nodes.map((n) => n.addr).includes(n.addr)))
                .sort((n1, n2) => n1.addr.localeCompare(n2.addr)),
            })
          }
          timeout = setTimeout(() => {
            getDetailsAndUpdate()
          }, POLLING_INTERVAL_MS)
        }
      } catch (error) {
        const fatalError: FatalError =
          typeof error === 'object' && Object.prototype.hasOwnProperty.call(error, 'shortMessage')
            ? (error as FatalError)
            : { shortMessage: safeErrorToStr(error) }
        setFatalError(fatalError)
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
  }, [data, state.key, setFatalError])

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
