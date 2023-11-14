/* eslint-disable react-hooks/exhaustive-deps */
import React, { useContext, useEffect, useMemo, useReducer, useRef, useState } from 'react'
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
  getTopicList,
  deleteTopic,
  publish,
} from '../util'
import {
  CreateUserKeyPairResponse,
  NodeType,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
  QueryResponse,
  UiNode,
  EventDiagnostic,
  TopicLsResponse,
  TopicDeleteResponse,
  PublishRequest,
  PublishResponse,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
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
// the below MUST be less than the above

export const reducer =
  () =>
  (state: AppState, action: AppAction): AppState => {
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
      case AppActionKey.ShowPreferences: {
        return { ...state, ...action, key: AppStateKey.Preferences }
      }
      case AppActionKey.ShowPublish: {
        return { ...state, ...action, key: AppStateKey.Publish }
      }
      case AppActionKey.ShowQuery: {
        return { ...state, ...action, key: AppStateKey.Query }
      }
      case AppActionKey.ShowSettings: {
        return { ...state, ...action, key: AppStateKey.Settings }
      }
      case AppActionKey.ShowTopics: {
        return { ...state, ...action, key: AppStateKey.Topics }
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

export interface Actions {
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
  publish: (args: { addr: string; events: PublishRequest['events'] }) => Promise<PublishResponse>
  setPublishState: React.Dispatch<React.SetStateAction<PublishState>>
  query: (args: { addr: string; query: string }) => Promise<QueryResponse>
  setQueryState: React.Dispatch<React.SetStateAction<QueryState>>
  setSettingPath: (path: string) => void
  setSettingJson: (json: string | null) => void
  getTopicList: (addr: string) => Promise<TopicLsResponse>
  deleteTopic: (addr: string, topic: string) => Promise<TopicDeleteResponse>
}

interface PublishState {
  node?: string
  tagsField: string
  payloadField: string
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
      publish: PublishState
      settings: SettingsState
    }
  | undefined
>(undefined)

export const AppStateProvider: React.FC<{
  setFatalError: (error: FatalError) => void
}> = ({ children, setFatalError }) => {
  const [state, dispatch] = useReducer(reducer(), {
    key: AppStateKey.Overview,
  })
  const [data, setData] = useState<Data>({
    nodes: [],
    offsets: none,
  })
  const [publishState, setPublishState] = useState<PublishState>({
    payloadField: '',
    tagsField: '',
  })
  const [queryState, setQueryState] = useState<QueryState>({ text: 'FROM allEvents', results: [] })
  const [settingsState, setSettingsState] = useState<SettingsState>({ path: '', json: null })

  const actions: Actions = {
    // Wrap addNodes and add the node as loading as soon as the request
    // is sent
    addNodes: (addrs) => {
      setData((current) => {
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
      setData((current) => ({
        ...current,
        nodes: current.nodes.filter((n) => !addrs.includes(n.addr)),
      }))
    },
    setSettings: (addr, settings, scope) => {
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : setSettings({ peer, settings, scope })
    },
    shutdownNode: (addr) => {
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : shutdownNode({ peer })
    },
    createUserKeyPair: (privateKeyPath) => {
      return createUserKeyPair({ privateKeyPath })
    },
    generateSwarmKey: () => {
      return generateSwarmKey({})
    },
    signAppManifest: ({ pathToManifest, pathToCertificate }) => {
      return signAppManifest({
        pathToManifest,
        pathToCertificate,
      })
    },
    publish: ({ addr, events }) => {
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : publish({ peer, events })
    },
    setPublishState,
    query: ({ addr, query: q }) => {
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
    getTopicList: function (
      addr: string,
    ): Promise<{ nodeId: string; activeTopic: string; topics: { [x: string]: number } }> {
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : getTopicList({ peer })
    },
    deleteTopic: function (
      addr: string,
      topic: string,
    ): Promise<{ nodeId: string; deleted: boolean }> {
      const peer = getPeer(addr, data)
      return peer === undefined
        ? Promise.reject(`not connected to ${addr}`)
        : deleteTopic({ peer, topic: topic })
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

  useNodeAutoUpdater({
    data,
    state,
    setData,
    setFatalError,
  })

  return (
    <AppStateContext.Provider
      value={{
        state,
        data,
        actions,
        dispatch,
        publish: publishState,
        query: queryState,
        settings: settingsState,
      }}
    >
      {children}
    </AppStateContext.Provider>
  )
}

// FIXME: replace this "global" locking mechanism with a per-node basis (require code restructuring)
// prevent concurrent autoupdater fn
const Locked: unique symbol = Symbol()
type Locked = typeof Locked
const Lock = () => {
  const internal = {
    locked: false,
  }

  const lock = {
    debugSymbol: Symbol(Math.round(Math.random() * 1000)),
    withLock: async <Fn extends () => Promise<Return>, Return extends unknown>(
      fn: Fn,
    ): Promise<Return | Locked> => {
      if (internal.locked) return Locked
      internal.locked = true
      try {
        return await fn()
      } catch (err) {
        console.error('Error thrown inside Lock', err)
        throw err
      } finally {
        internal.locked = false
      }
    },
  }

  return lock
}

const useNodeAutoUpdater = ({
  data,
  state,
  setData,
  setFatalError,
}: {
  data: Data
  state: AppState
  setData: React.Dispatch<React.SetStateAction<Data>>
  setFatalError: (error: FatalError) => void
}) => {
  const store = useStore()

  // FIXME: Mechanism moved here from NodesOverview to prevent data race
  const favoriteNodeAddrs = store.key === 'Loaded' ? store.data.preferences.favoriteNodeAddrs : []

  const lock = useMemo(Lock, [])

  useEffect(() => {
    let mounted = true
    if (store.key !== StoreStateKey.Loaded) {
      return
    }

    const getTimeoutSec =
      (store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout) ||
      DEFAULT_TIMEOUT_SEC

    const sleep = (duration: number = POLLING_INTERVAL_MS) =>
      new Promise((resolve) => setTimeout(resolve, duration))

    const autoUpdater = (async () => {
      while (mounted) {
        const condition = await lock.withLock(async () => {
          if (state.key === 'SetupUserKey') {
            await sleep(POLLING_INTERVAL_MS)
            return
          }
          // Connect
          // =======
          const connectibleNodes = filterConnectibleNodes(data.nodes)
          const connectibleNodesAddrs = new Set(connectibleNodes.map((n) => n.addr))
          setData((current) => ({
            ...current,
            nodes: current.nodes.map((n) =>
              connectibleNodesAddrs.has(n.addr)
                ? {
                    type: NodeType.Connecting,
                    addr: n.addr,
                    prevError: n.type === NodeType.Unreachable ? n.error : null,
                  }
                : n,
            ),
          }))

          const connectionResultsMap = await attemptConnections(connectibleNodes, getTimeoutSec)
          setData((current) => ({
            ...current,
            nodes: current.nodes.map((n) => {
              const result = connectionResultsMap.get(n.addr)
              if (result === undefined) return n
              if (result.type === 'success') {
                const { addr, peer } = result
                console.log('connected to', addr, peer)
                return { type: NodeType.Connected, addr, peer }
              } else {
                const { addr, err } = result
                console.log('connect error', addr, err)
                const error = safeErrorToStr(err)
                return { type: NodeType.Unreachable, addr, error }
              }
            }),
          }))

          // Get Update
          // ==========

          const newNodes = await retrieveNodeInfos(data.nodes, getTimeoutSec)
          const offsetsInfo = OffsetInfo.of(newNodes)
          const nodes = mergeNodeInfo(data.nodes, newNodes, favoriteNodeAddrs)

          if (!deepEqual(data.nodes, nodes) || !deepEqual(data.offsets, some(offsetsInfo))) {
            console.log(`+++ updating app-state/nodes +++`, nodes.map((n) => n.addr).join(', '))
            // FIXME data race between node removal and
            // node update happens here because data.nodes can be mutated when
            // this function runs
            setData({
              offsets: some(offsetsInfo),
              nodes,
            })
          }

          console.log('after', data.nodes.map((x) => x.addr).join(','))
          await sleep(POLLING_INTERVAL_MS)
        })

        if (condition === Locked) {
          await sleep(POLLING_INTERVAL_MS)
        }
      }
    })()

    autoUpdater.catch((error) => {
      const fatalError: FatalError =
        typeof error === 'object' && Object.prototype.hasOwnProperty.call(error, 'shortMessage')
          ? (error as FatalError)
          : { shortMessage: safeErrorToStr(error) }
      setFatalError(fatalError)

      // FIXME: this triggers rerender, which will auto-update
      setData((current) => ({
        ...current,
      }))
    })

    return () => {
      // FIXME: unmounted kept being called because `data` is listed as the useEffect's dependency list
      mounted = false
    }

    // The following line generates a warning; this is known; please don't fix without
    // ensuring that there are no unnecessary re-renders.
  }, [data, state.key, setFatalError, store.key, favoriteNodeAddrs])
}

export const useAppState = () => {
  const c = useContext(AppStateContext)
  if (c === undefined) {
    throw 'AppStateContext is undefined'
  }
  return c
}

// Utils

const SHOULD_CONNECT_WHEN_OF_TYPE = Object.freeze(
  new Set([NodeType.Disconnected, NodeType.Fresh, NodeType.Unreachable]),
)
const filterConnectibleNodes = (nodes: UiNode[]) =>
  nodes.filter((n) => SHOULD_CONNECT_WHEN_OF_TYPE.has(n.type))

const attemptConnections = (connectibleNodes: UiNode[], timeout: number) =>
  Promise.all(
    connectibleNodes.map(({ addr }) =>
      connect({ addr, timeout })
        .then(({ peer }) => ({ type: 'success', addr, peer } as const))
        .catch((err) => ({ type: 'error', addr, err } as const)),
    ),
  ).then((acc) => new Map(acc.map((result) => [result.addr, result])))

const retrieveNodeInfos = (nodes: UiNode[], timeout: number) =>
  Promise.all(
    nodes
      .map((n) => ('peer' in n ? { peer: n.peer, n } : null))
      .filter(
        (pair): pair is Exclude<typeof pair, null> =>
          pair !== null && pair.n.type !== NodeType.Disconnected,
      )
      .map(({ peer, n }) =>
        getNodeDetails({ peer, timeout })
          .then((res) => ({ ...res, timeouts: 0, addr: n.addr }))
          .catch(() => ({
            ...n,
            timeouts: n.type === NodeType.Reachable ? n.timeouts + 1 : 0,
          })),
      ),
  )

const mergeNodeInfo = (currentNodes: UiNode[], newNodes: UiNode[], favoriteNodeAddrs: string[]) => {
  const currentAddrs = new Set(currentNodes.map((n) => n.addr))
  const newAddrs = new Set(newNodes.map((n) => n.addr))
  const oldNodes = currentNodes.filter((n) => !newAddrs.has(n.addr)) // the ones that didn’t get retrieved
  const newAndUnremovedNodes = newNodes.filter((n) => currentAddrs.has(n.addr)) // cull the removed ones

  const allNodes = oldNodes.concat(newAndUnremovedNodes)

  // Patch favorite nodes
  // FIXME this is moved here because of unidentified data race when put in the node page
  const finalNodesAddrs = new Set(allNodes.map((x) => x.addr))
  const favoriteNodes = favoriteNodeAddrs
    .filter((addr) => !finalNodesAddrs.has(addr))
    .map(
      (addr): UiNode => ({
        type: NodeType.Fresh,
        addr,
      }),
    )

  return allNodes.concat(favoriteNodes).sort((n1, n2) => n1.addr.localeCompare(n2.addr))
}
