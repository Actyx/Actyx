import React, { useEffect, useState } from 'react'
import { hot } from 'react-hot-loader'
import {
  NodesOverview,
  NodeDetail,
  About,
  NodeAuth,
  AppSigning,
  FatalError,
  SetupUserKey,
  Diagnostics,
  SwarmKey,
  Preferences,
  Publish,
  Query,
  Topics,
  Settings,
} from './screens'
import { AppStateProvider, useAppState, AppStateKey } from './app-state'
import '../index.css'
import { waitForFatalError } from './util'
import { FatalError as FatalErrorT } from '../common/ipc'
import { StoreProvider } from './store'

const Root = () => {
  const [fatalError, setFatalError] = useState<null | FatalErrorT>(null)

  useEffect(() => {
    ;(async () => {
      setFatalError(await waitForFatalError())
    })()
  }, [])

  return (
    <StoreProvider>
        <AppStateProvider setFatalError={setFatalError}>
          {fatalError !== null ? (
            <FatalError error={fatalError} />
          ) : (
            <div className="h-full w-full max-h-screen max-w-screen overflow-hidden">
              <div className="bg-gray-100 p-0 h-full">
                <Content />
              </div>
            </div>
          )}
        </AppStateProvider>
    </StoreProvider>
  )
}

const Content: React.FC = () => {
  const { state } = useAppState()

  switch (state.key) {
    case AppStateKey.Overview:
      return <NodesOverview />
    case AppStateKey.SetupUserKey:
      return <SetupUserKey />
    case AppStateKey.NodeDetail:
      return <NodeDetail {...state} />
    case AppStateKey.About:
      return <About />
    case AppStateKey.NodeAuth:
      return <NodeAuth />
    case AppStateKey.AppSigning:
      return <AppSigning />
    case AppStateKey.Diagnostics:
      return <Diagnostics />
    case AppStateKey.Preferences:
      return <Preferences />
    case AppStateKey.SwarmKey:
      return <SwarmKey />
    case AppStateKey.Publish:
      return <Publish />
    case AppStateKey.Query:
      return <Query />
    case AppStateKey.Settings:
      return <Settings />
    case AppStateKey.Topics:
      return <Topics />
  }
}

export default hot(module)(Root)
