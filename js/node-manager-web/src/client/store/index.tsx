import React, { useContext, useEffect, useReducer, useState } from 'react'
import { IpcFromClient, IpcToClient } from '../../common/ipc'
import { StoreData as Data } from '../../common/types'
import { StoreAction, StoreActionKey, StoreState, StoreStateKey } from './types'

const reducer = (state: StoreState, action: StoreAction): StoreState => {
  switch (action.key) {
    case StoreActionKey.LoadOrSave: {
      return { ...state, key: StoreStateKey.LoadingOrSaving }
    }
    case StoreActionKey.HasLoaded: {
      return { ...action, key: StoreStateKey.Loaded }
    }
  }
}

const saveAndReloadDataViaIpc = (new_data: Data | null, onData: (data: Data) => void) => {
  // ipcRenderer.once(IpcToClient.StoreLoaded, (event, arg) => {
  //   onData(arg as Data)
  // })
  // ipcRenderer.send(IpcFromClient.LoadStore, new_data)
  onData({ analytics: { disabled: true, userId: 'dev' }, preferences: { favoriteNodeAddrs: [] } })
}

const Context = React.createContext<StoreState | undefined>(undefined)

export const StoreProvider: React.FC<{}> = ({ children }) => {
  const [state, dispatch] = useReducer(reducer, { key: StoreStateKey.Initial })

  const updateAndReload = (data: Data | null) => {
    saveAndReloadDataViaIpc(data, (data) => {
      dispatch({
        key: StoreActionKey.HasLoaded,
        data,
        actions: {
          reload: () => updateAndReload(null),
          updateAndReload,
        },
      })
    })
  }

  useEffect(() => {
    ;(async () => {
      if (state.key === 'Initial') {
        updateAndReload(null)
      }
    })()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.key])

  return <Context.Provider value={state}>{children}</Context.Provider>
}

export const useStore = () => {
  const c = useContext(Context)
  if (c === undefined) {
    throw 'Store context is undefined'
  }
  return c
}
