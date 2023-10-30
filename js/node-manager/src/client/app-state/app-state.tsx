/* eslint-disable react-hooks/exhaustive-deps */
import React, { useContext, useEffect, useMemo, useReducer, useState } from 'react'
import {
  signAppManifest,
  createUserKeyPair,
  generateSwarmKey,
  waitForNoUserKeysFound,
} from '../util'
import {
  CreateUserKeyPairResponse,
  GenerateSwarmKeyResponse,
  SignAppManifestResponse,
  EventDiagnostic,
} from '../../common/types'
import { AppState, AppAction, AppStateKey, AppActionKey } from './types'
import { FatalError } from '../../common/ipc'
import { ObsValcon } from '../util/valcon'
import { ServReact } from '../util/serv-react'
import { FavoriteParams, NodeManagerAgent, NodeManagerAgentContext } from '../agents/node-manager'
import { useStore } from '../store'

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
export interface Actions {
  createUserKeyPair: (privateKeyPath: string | null) => Promise<CreateUserKeyPairResponse>
  generateSwarmKey: () => Promise<GenerateSwarmKeyResponse>
  signAppManifest: ({
    pathToManifest,
    pathToCertificate,
  }: {
    pathToManifest: string
    pathToCertificate: string
  }) => Promise<SignAppManifestResponse>
  setPublishState: React.Dispatch<React.SetStateAction<PublishState>>
  setQueryState: React.Dispatch<React.SetStateAction<QueryState>>
  setSettingPath: (path: string) => void
  setSettingJson: (json: string | null) => void
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
      actions: Actions
      dispatch: AppDispatch
      query: QueryState
      publish: PublishState
      settings: SettingsState
    }
  | undefined
>(undefined)

// const toggleFavorite = () => {
//   isFavorite(store, node.addr) ? unmkFavorite() : mkFavorite()
// }

// const mkFavorite = () => {
//   if (store.key !== 'Loaded') {
//     return
//   }
//   store.actions.updateAndReload({
//     ...store.data,
//     preferences: {
//       ...store.data.preferences,
//       favoriteNodeAddrs: store.data.preferences.favoriteNodeAddrs.concat([node.addr]),
//     },
//   })
// }

// const unmkFavorite = () => {
//   if (store.key !== 'Loaded') {
//     return
//   }
//   store.actions.updateAndReload({
//     ...store.data,
//     preferences: {
//       ...store.data.preferences,
//       favoriteNodeAddrs: store.data.preferences.favoriteNodeAddrs.filter(
//         (addr) => addr !== node.addr,
//       ),
//     },
//   })
// }

export const AppStateProvider: React.FC<{
  setFatalError: (error: FatalError) => void
}> = ({ children, setFatalError }) => {
  const [state, dispatch] = useReducer(reducer(), {
    key: AppStateKey.Overview,
  })
  const [publishState, setPublishState] = useState<PublishState>({
    payloadField: '',
    tagsField: '',
  })
  const [queryState, setQueryState] = useState<QueryState>({ text: 'FROM allEvents', results: [] })
  const [settingsState, setSettingsState] = useState<SettingsState>({ path: '', json: null })
  const actions: Actions = {
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
    setPublishState,
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
  const store = useStore()

  useEffect(() => {
    ;(async () => {
      await waitForNoUserKeysFound()
      dispatch({ key: AppActionKey.ShowSetupUserKey })
    })()
  }, [])

  // NodeManagerAgent

  // This ObsValcon bridge the "snapshot" nature of react state into
  // getCurrent-able container
  const nodeManagerAgentAllowedToWorkRef = useMemo(() => ObsValcon(false), [])
  const nodeManagerAgentAllowedToWork = state.key !== 'SetupUserKey'
  useEffect(() => {
    nodeManagerAgentAllowedToWorkRef.set(nodeManagerAgentAllowedToWork)
  }, [nodeManagerAgentAllowedToWork])

  const favoriteAddressesControlRef = useMemo<FavoriteParams>(
    () =>
      ObsValcon<null | {
        initial: string[]
        set: (_: string[]) => unknown
      }>(null),
    [],
  )
  useEffect(() => {
    if (store.key === 'Loaded') {
      favoriteAddressesControlRef.set({
        initial: store.data.preferences.favoriteNodeAddrs,
        set: (favorite) =>
          store.actions.updateAndReload({
            ...store.data,
            preferences: {
              ...store.data.preferences,
              favoriteNodeAddrs: favorite,
            },
          }),
      })
    }
  }, [store])

  const nodeTimeoutRef = useMemo<ObsValcon<number | null>>(() => ObsValcon<null | number>(null), [])
  const nodeTimeout = (store.key === 'Loaded' && store.data.preferences.nodeTimeout) || null
  useEffect(() => {
    nodeTimeoutRef.set(nodeTimeout)
  }, [nodeTimeout])

  // Initialize NodeManagerAgent here
  const nodeManagerAgent = ServReact.useOwned(() =>
    NodeManagerAgent({
      allowedToWorkRef: nodeManagerAgentAllowedToWorkRef,
      favoriteParams: favoriteAddressesControlRef,
      timeoutRef: nodeTimeoutRef,
    }),
  )

  return (
    <NodeManagerAgentContext.Provider value={nodeManagerAgent}>
      <AppStateContext.Provider
        value={{
          state,
          actions,
          dispatch,
          publish: publishState,
          query: queryState,
          settings: settingsState,
        }}
      >
        {children}
      </AppStateContext.Provider>
    </NodeManagerAgentContext.Provider>
  )
}
export const useAppState = () => {
  const c = useContext(AppStateContext)
  if (c === undefined) {
    throw 'AppStateContext is undefined'
  }
  return c
}
