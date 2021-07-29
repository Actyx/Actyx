export const enum AppStateKey {
  Overview = 'Overview',
  SetupUserKey = 'SetupUserKey',
  About = 'About',
  Diagnostics = 'Diagnostics',
  AppSigning = 'AppSigning',
  NodeAuth = 'NodeAuth',
  SwarmKey = 'SwarmKey',
  NodeDetail = 'NodeDetail',
  Preferences = 'Preferences',
  Query = 'Query',
}

interface Overview {
  key: AppStateKey.Overview
}

interface SetupUserKey {
  key: AppStateKey.SetupUserKey
}

interface About {
  key: AppStateKey.About
}

interface Diagnostics {
  key: AppStateKey.Diagnostics
}

interface AppSigning {
  key: AppStateKey.AppSigning
}

interface NodeAuth {
  key: AppStateKey.NodeAuth
}

interface SwarmKey {
  key: AppStateKey.SwarmKey
}

interface Preferences {
  key: AppStateKey.Preferences
}

interface Query {
  key: AppStateKey.Query
}

interface NodeDetail {
  key: AppStateKey.NodeDetail
  addr: string
}

export type AppState =
  | Overview
  | NodeDetail
  | About
  | NodeAuth
  | AppSigning
  | SetupUserKey
  | Diagnostics
  | SwarmKey
  | Preferences
  | Query

export enum AppActionKey {
  ShowOverview = 'ShowOverview',
  ShowSetupUserKey = 'ShowSetupUserKey',
  ShowAppSigning = 'ShowAppSigning',
  ShowNodeDetail = 'ShowNodeDetail',
  ShowAbout = 'ShowAbout',
  ShowGenerateSwarmKey = 'ShowGenerateSwarmKey',
  ShowDiagnostics = 'ShowDiagnostics',
  ShowNodeAuth = 'ShowNodeAuth',
  ShowPreferences = 'ShowPreferences',
  ShowQuery = 'ShowQuery',
}
interface ShowOverview {
  key: AppActionKey.ShowOverview
}

interface ShowPreferences {
  key: AppActionKey.ShowPreferences
}
interface ShowSetupUserKey {
  key: AppActionKey.ShowSetupUserKey
}

interface ShowAppSigning {
  key: AppActionKey.ShowAppSigning
}

interface ShowNodeDetail {
  key: AppActionKey.ShowNodeDetail
  addr: string
}

interface ShowAbout {
  key: AppActionKey.ShowAbout
}

interface ShowGenerateSwarmKey {
  key: AppActionKey.ShowGenerateSwarmKey
}

interface ShowDiagnostics {
  key: AppActionKey.ShowDiagnostics
}

interface ShowQuery {
  key: AppActionKey.ShowQuery
}

interface ShowNodeAuth {
  key: AppActionKey.ShowNodeAuth
}

export type AppAction =
  | ShowOverview
  | ShowSetupUserKey
  | ShowAppSigning
  | ShowNodeDetail
  | ShowNodeAuth
  | ShowAbout
  | ShowDiagnostics
  | ShowGenerateSwarmKey
  | ShowPreferences
  | ShowQuery
