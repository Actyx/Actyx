import * as native from '../../../../../rust/actyx/node-manager-bindings/node-manager-bindings.node'
export const connect = native.connect.bind(native)
export const getNodeDetails = native.getNodeDetails.bind(native)
export const createUserKeyPair = native.createUserKeyPair.bind(native)
export const setSettings = native.setSettings.bind(native)
export const generateSwarmKey = native.generateSwarmKey.bind(native)
export const signAppManifest = native.signAppManifest.bind(native)
export const shutdown = native.shutdown.bind(native)
export const query = native.query.bind(native)
export const onDisconnect = native.onDisconnect.bind(native)
export type AsyncTask = native.AsyncTask
