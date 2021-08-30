import React, { useContext, useEffect, useState } from "react"
import { AnalyticsActions, AnalyticsEvent } from "./types"
import { StoreStateKey } from "../store/types"
import { useStore } from "../store"
import amplitude, { AmplitudeClient } from "amplitude-js"
import { AMPLITUDE_ANALYTICS_API_KEY } from "../../common/analytics"
import { getPackageVersion } from "../util"

const Context = React.createContext<State | undefined>(undefined)

const sanitizeStr = (str: string) => str.replace(/((\/|\\)\S*\s)/g, "******")

const mkAnalyticsActions = (
  client: AmplitudeClient | null
): AnalyticsActions => {
  const logEvent = (event: AnalyticsEvent) => {
    if (client === null) {
      if (window !== undefined && (window as any).AX_LOG_ANALYTICS_EVENTS) {
        console.log(
          `not publishing analytics event (client=null; probably disabled)`
        )
      }
      return
    }
    if (window !== undefined && (window as any).AX_LOG_ANALYTICS_EVENTS) {
      console.log(`analytics event:`)
      console.log({ key: event.key, event })
    }
    client.logEvent(event.key, event)
  }
  return {
    viewedScreen: (name) => logEvent({ key: "ViewedScreen", name }),
    queriedEvents: (query) => logEvent({ key: "QueriedEvents", query }),
    startedApp: () => logEvent({ key: "StartedApp" }),
    addedNode: () => logEvent({ key: "AddedNode" }),
    removedNode: () => logEvent({ key: "RemovedNode" }),
    removedAllNodes: () => logEvent({ key: "RemovedAllNodes" }),
    signedAppManifest: () => logEvent({ key: "SignedAppManifest" }),
    setSettings: () => logEvent({ key: "SetSettings" }),
    shutdownNode: () => logEvent({ key: "ShutdownNode" }),
    generatedSwarmKey: () => logEvent({ key: "GeneratedSwarmKey" }),
    createdUserKeyPair: () =>
      logEvent({ key: "CreatedUserKeyPair", wasDefault: false }),
    gotFatalError: ({ shortMessage, details }) =>
      logEvent({
        key: "GotFatalError",
        shortMessage: sanitizeStr(shortMessage),
        details: details ? sanitizeStr(details) : undefined,
      }),
    gotError: (error) =>
      logEvent({ key: "GotError", shortMessage: sanitizeStr(error) }),
  }
}

type State =
  | { key: "loading" }
  | { key: "disabled"; actions: AnalyticsActions }
  | { key: "active"; actions: AnalyticsActions }

export const Provider: React.FC<{}> = ({ children }) => {
  const [state, setState] = useState<State>({ key: "loading" })
  const store = useStore()

  const analyticsDisabled =
    store.key === StoreStateKey.Loaded && store.data.analytics.disabled
  const analyticsUserId =
    store.key === StoreStateKey.Loaded && store.data.analytics.userId

  useEffect(() => {
    if (store.key !== StoreStateKey.Loaded) {
      return
    }

    setState(() => {
      if (analyticsDisabled) {
        console.log(`not setting up analytics since disabled by user`)
        return { key: "disabled", actions: mkAnalyticsActions(null) }
      }

      if (!analyticsUserId) {
        throw new Error(`analytics.userId unexpectedly empty`)
      }

      const client = amplitude.getInstance()
      client.init(AMPLITUDE_ANALYTICS_API_KEY, analyticsUserId, {
        trackingOptions: {
          ip_address: false,
        },
      })
      client.setVersionName(getPackageVersion())
      const actions = mkAnalyticsActions(client)
      return { key: "active", actions }
    })
  }, [state.key, store.key, analyticsDisabled, analyticsUserId])

  return <Context.Provider value={state}>{children}</Context.Provider>
}

export const useAnalytics = () => {
  const c = useContext(Context)
  if (c === undefined) {
    throw "Analytics context is undefined"
  }
  return c.key !== "loading" ? c.actions : undefined
}
