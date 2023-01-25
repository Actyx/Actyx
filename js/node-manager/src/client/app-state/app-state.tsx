/* eslint-disable react-hooks/exhaustive-deps */
import React, { useContext, useEffect, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  getNodeDetails,
  setSettings,
  waitForNoUserKeysFound,
  shutdownNode,
  query,
  connect,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  Node,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
  QueryResponse,
  UiNode,
  EventDiagnostic,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
import { useAnalytics } from '../analytics'
import { AnalyticsActions } from '../analytics/types'
import { FatalError } from '../../common/ipc'
import { safeErrorToStr } from '../../common/util'
import deepEqual from 'fast-deep-equal'
import { OffsetInfo } from '../offsets'
import { none, Option, some } from 'fp-ts/lib/Option'
import { useStore } from '../store'
import { StoreStateKey } from '../store/types'
import { DEFAULT_TIMEOUT_SEC } from 'common/consts'
import { ipcRenderer } from 'electron'

const POLLING_INTERVAL_MS = 1_000
const DEFER_UPDATE_AFTER_CHANGE_MS = 200
// the below MUST be less than the above
const DEFER_CONNECTING_STATE_MS = 150

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
      case AppActionKey.ShowSettings: {
        if (analytics) analytics.viewedScreen('Settings')
        return { ...state, ...action, key: AppStateKey.Settings }
      }
    }
  }

interface Data {
  nodes: UiNode[]
  offsets: Option<OffsetInfo>
}

const getPeer = (addr: string, data: Data) => {
  const n = data.nodes.find((n) => n.addr === addr)
  return n && 'peer' in n ? n.peer : undefined
}

interface Actions {
  addNodes: (addrs: string[]) => void
  remNodes: (addrs: string[]) => void
  setSettings: (addr: string, settings: object, scope: string[]) => Promise<void>
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
  query: (args: { addr: string; query: string }) => Promise<QueryResponse>
  setQueryState: React.Dispatch<React.SetStateAction<QueryState>>
  setSettingPath: (path: string) => void
  setSettingJson: (json: string | null) => void
}

interface QueryState {
  text: string
  node?: string
  results: EventDiagnostic[]
}

interface SettingsState {
  path: string
  json: string | null
}

export type AppDispatch = (action: AppAction) => void
const AppStateContext = React.createContext<
  | {
      state: AppState
      data: Data
      actions: Actions
      dispatch: AppDispatch
      query: QueryState
      settings: SettingsState
    }
  | undefined
>(undefined)

export const AppStateProvider: React.FC<{
  setFatalError: (error: FatalError) => void
}> = ({ children, setFatalError }) => {
  const analytics = useAnalytics()
  const store = useStore()
  const [state, dispatch] = useReducer(reducer(analytics), {
    key: AppStateKey.Overview,
  })
  const [data, setData] = useState<Data>({
    nodes: [],
    offsets: none,
  })
  const [queryState, setQueryState] = useState<QueryState>({ text: 'FROM allEvents', results: [] })
  const [settingsState, setSettingsState] = useState<SettingsState>({ path: '', json: null })

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => {
        if (analytics) {
          addrs.forEach((addr) => {
            analytics.addedNode()
          })
        }
        return {
          ...current,
          nodes: current.nodes.concat(
            addrs.map((addr) => ({
              type: NodeType.Fresh,
              addr,
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
    setSettings: (addr, settings, scope) => {
      if (analytics) {
        analytics.setSettings()
      }
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : setSettings({ peer, settings, scope })
    },
    shutdownNode: (addr) => {
      if (analytics) {
        analytics.shutdownNode()
      }
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : shutdownNode({ peer })
    },
    createUserKeyPair: (privateKeyPath) => {
      if (analytics) {
        analytics.createdUserKeyPair(privateKeyPath === null)
      }
      return createUserKeyPair({ privateKeyPath })
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
    query: ({ addr, query: q }) => {
      if (analytics) {
        analytics.queriedEvents(q)
      }
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : query({ peer, query: q })
    },
    setQueryState,
    setSettingPath: (path) => {
      setSettingsState((current) => {
        if (path === current.path) return current
        return { ...current, path }
      })
    },
    setSettingJson: (json) => {
      setSettingsState((current) => {
        if (json === current.json) return current
        return { ...current, json }
      })
    },
  }

  useEffect(() => {
    ipcRenderer.on('onDisconnect', (event, peer) => {
      console.log('onDisconnect', event, peer)
      setData((current) => {
        return {
          ...current,
          nodes: current.nodes.map((n) =>
            'peer' in n && n.peer === peer
              ? { type: NodeType.Disconnected, peer, addr: n.addr }
              : n,
          ),
        }
      })
    })
  }, [])

  useEffect(() => {
    ;(async () => {
      await waitForNoUserKeysFound()
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    })()
  }, [])

  useEffect(() => {
    let unmounted = false
    if (store.key !== StoreStateKey.Loaded) {
      return
    }

    let timeout: ReturnType<typeof setTimeout> | null = null
    const getTimeoutSec =
      (store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout) ||
      DEFAULT_TIMEOUT_SEC
    const getDetailsAndUpdate = async () => {
      console.log('getting node information')
      try {
        data.nodes.forEach((n) => {
          switch (n.type) {
            case NodeType.Disconnected:
            case NodeType.Fresh:
            case NodeType.Unreachable: {
              const addr = n.addr
              console.log('connecting to', addr)

              // defer setting `connecting` to avoid flickering when it fails fast
              const prevError = n.type === NodeType.Unreachable ? n.error : null
              const setter = setTimeout(() => {
                console.log('setting connecting status', addr)
                setData((current) => ({
                  ...current,
                  nodes: current.nodes.map((n) =>
                    n.addr === addr ? { type: NodeType.Connecting, addr, prevError } : n,
                  ),
                }))
              }, DEFER_CONNECTING_STATE_MS)

              connect({ addr, timeout: getTimeoutSec })
                .then(({ peer }) => {
                  console.log('connected to', addr, peer)
                  clearTimeout(setter)
                  setData((current) => ({
                    ...current,
                    nodes: current.nodes.map((n) =>
                      n.addr === addr ? { type: NodeType.Connected, addr, peer } : n,
                    ),
                  }))
                })
                .catch((err) => {
                  console.log('connect error', addr, err)
                  clearTimeout(setter)
                  const idx = data.nodes.findIndex((n) => n.addr === addr)
                  if (idx >= 0) {
                    const node = data.nodes[idx]
                    const error = safeErrorToStr(err)
                    // do not update state without need, to avoid causing too quick retries
                    if (node.type !== NodeType.Unreachable || node.error !== error) {
                      setData((current) => {
                        const nodes = current.nodes.slice()
                        console.log('overwriting', nodes[idx])
                        nodes[idx] = { type: NodeType.Unreachable, addr, error }
                        return { ...current, nodes }
                      })
                    }
                  }
                })
            }
          }
        })

        const nodeInfos = await Promise.all(
          data.nodes.reduce((acc: Promise<UiNode>[], n) => {
            if ('peer' in n && n.type !== NodeType.Disconnected) {
              acc.push(
                getNodeDetails({ peer: n.peer, timeout: getTimeoutSec })
                  .then((res) => ({
                    ...res,
                    timeouts: 0,
                    addr: n.addr,
                  }))
                  .catch(() => ({
                    ...n,
                    timeouts: n.type === NodeType.Reachable ? n.timeouts + 1 : 0,
                  })),
              )
            }
            return acc
          }, []),
        )

        const offsetsInfo = OffsetInfo.of(nodeInfos)
        const nodes = data.nodes
          .filter((n) => nodeInfos.every((n2) => n.addr !== n2.addr)) // the ones that didn’t get retrieved
          .concat(nodeInfos.filter((n) => data.nodes.some((n2) => n.addr === n2.addr))) // cull the removed ones
          .sort((n1, n2) => n1.addr.localeCompare(n2.addr))
        if (!deepEqual(data.nodes, nodes) || !deepEqual(data.offsets, some(offsetsInfo))) {
          console.log(`+++ updating app-state/nodes +++`)
          setData({
            offsets: some(offsetsInfo),
            nodes,
          })
        }
        if (!unmounted) {
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
      timeout = setTimeout(getDetailsAndUpdate, DEFER_UPDATE_AFTER_CHANGE_MS)
    }

    return () => {
      unmounted = true
      if (timeout !== null) {
        clearTimeout(timeout)
      }
    }

    // The following line generates a warning; this is known; please don't fix without
    // ensuring that there are no unnecessary re-renders.
  }, [data, state.key, setFatalError, store.key])

  return (
    <AppStateContext.Provider
      value={{ state, data, actions, dispatch, query: queryState, settings: settingsState }}
    >
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
