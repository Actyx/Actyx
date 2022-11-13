declare module '*node-manager-bindings.node' {
  export type AsyncTask = (json: string, cb: (err: string, json: string) => void) => undefined
  export const connect: AsyncTask
  export const getNodeDetails: AsyncTask
  export const setSettings: AsyncTask
  export const createUserKeyPair: AsyncTask
  export const generateSwarmKey: AsyncTask
  export const signAppManifest: AsyncTask
  export const shutdown: AsyncTask
  export const query: AsyncTask
  export const onDisconnect: (cb: (peer: string) => void) => undefined
}
