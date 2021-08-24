import { FatalError } from "../../common/ipc";

export interface AnalyticsActions {
  viewedScreen: (screenName: string) => void;
  queriedEvents: (query: string) => void;
  startedApp: () => void;
  addedNode: () => void;
  removedNode: () => void;
  removedAllNodes: () => void;
  signedAppManifest: () => void;
  setSettings: () => void;
  shutdownNode: () => void;
  generatedSwarmKey: () => void;
  createdUserKeyPair: () => void;
  gotFatalError: (error: FatalError) => void;
  gotError: (errorMsg: string) => void;
}

export type AnalyticsEvent =
  | { readonly key: "ViewedScreen"; name: string }
  | { readonly key: "QueriedEvents"; query: string }
  | { readonly key: "StartedApp" }
  | { readonly key: "AddedNode" }
  | { readonly key: "RemovedNode" }
  | { readonly key: "RemovedAllNodes" }
  | { readonly key: "SignedAppManifest" }
  | { readonly key: "CreatedUserKeyPair"; wasDefault: boolean }
  | { readonly key: "GotFatalError"; shortMessage: string; details?: string }
  | { readonly key: "GotError"; shortMessage: string }
  | { readonly key: "SetSettings" }
  | { readonly key: "ShutdownNode" }
  | { readonly key: "GeneratedSwarmKey" };
