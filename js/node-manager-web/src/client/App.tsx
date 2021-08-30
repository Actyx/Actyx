import React, { useEffect, useState } from "react"
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
  Query,
} from "./screens"
import { AppStateProvider, useAppState, AppStateKey } from "./app-state"
import { Provider as AnalyticsProvider, useAnalytics } from "./analytics"
import "../index.css"
import { waitForFatalError } from "./util"
import { FatalError as FatalErrorT } from "../common/ipc"
import { StoreProvider } from "./store"

const Root = () => {
  const [fatalError, setFatalError] = useState<null | FatalErrorT>(null)

  useEffect(() => {
    ;(async () => {
      setFatalError(await waitForFatalError())
    })()
  }, [])

  return (
    <StoreProvider>
      <AnalyticsProvider>
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
      </AnalyticsProvider>
    </StoreProvider>
  )
}

const Content: React.FC = () => {
  const { state } = useAppState()
  const [haveLoggedStartup, setHaveLoggedStartup] = useState(false)
  const analytics = useAnalytics()

  useEffect(() => {
    if (analytics) {
      setHaveLoggedStartup((haveLogged) => {
        if (!haveLogged) {
          analytics.startedApp()
        }
        return true
      })
    }
  }, [analytics, haveLoggedStartup])
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
    case AppStateKey.Query:
      return <Query />
  }
}

export default Root
