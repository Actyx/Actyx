export const enum AppStateKey {
  Overview = 'Overview',
  SetupUserKey = 'SetupUserKey',
  About = 'About',
  Diagnostics = 'Diagnostics',
  AppSigning = 'AppSigning',
  NodeAuth = 'NodeAuth',
  SwarmKey = 'SwarmKey',
  NodeDetail = 'NodeDetail',
}

interface Overview {
  key: AppStateKey.Overview
  version: string
}

interface SetupUserKey {
  key: AppStateKey.SetupUserKey
  version: string
}

interface About {
  key: AppStateKey.About
  version: string
}

interface Diagnostics {
  key: AppStateKey.Diagnostics
  version: string
}

interface AppSigning {
  key: AppStateKey.AppSigning
  version: string
}

interface NodeAuth {
  key: AppStateKey.NodeAuth
  version: string
}

interface SwarmKey {
  key: AppStateKey.SwarmKey
  version: string
}

interface NodeDetail {
  key: AppStateKey.NodeDetail
  version: string
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

export enum AppActionKey {
  ShowOverview = 'ShowOverview',
  ShowSetupUserKey = 'ShowSetupUserKey',
  ShowAppSigning = 'ShowAppSigning',
  ShowNodeDetail = 'ShowNodeDetail',
  ShowAbout = 'ShowAbout',
  ShowGenerateSwarmKey = 'ShowGenerateSwarmKey',
  ShowDiagnostics = 'ShowDiagnostics',
  ShowNodeAuth = 'ShowNodeAuth',
  SetVersion = 'SetVersion',
}
interface ShowOverview {
  key: AppActionKey.ShowOverview
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

interface ShowNodeAuth {
  key: AppActionKey.ShowNodeAuth
}

interface SetVersion {
  key: AppActionKey.SetVersion
  version: string
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
  | SetVersion
