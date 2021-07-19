import * as io from 'io-ts'

// Disk store

export const StoreData = io.type({
  preferences: io.type({
    favoriteNodeAddrs: io.array(io.string),
  }),
  analytics: io.type({
    disabled: io.boolean,
    userId: io.string,
  }),
})

export type StoreData = io.TypeOf<typeof StoreData>

// Basics

export const Peer = io.type({
  peerId: io.string,
  addrs: io.array(io.string),
})

export type Peer = io.TypeOf<typeof Peer>

export const Connection = io.type({
  peerId: io.string,
  addr: io.string,
})

export type Connection = io.TypeOf<typeof Connection>

export const SwarmOffsets = io.type({
  present: io.record(io.string, io.Int),
  toReplicate: io.record(io.string, io.Int),
})
export type SwarmOffsets = io.TypeOf<typeof SwarmOffsets>

export const NodeSwarmState = io.type({
  peerId: io.string,
  swarmAddrs: io.array(io.string),
  announceAddrs: io.array(io.string),
  adminAddrs: io.array(io.string),
  connections: io.array(Connection),
  knownPeers: io.array(Peer),
})

export type NodeSwarmState = io.TypeOf<typeof NodeSwarmState>

export const enum NodeType {
  Reachable = 'reachableNode',
  Unauthorized = 'unauthorizedNode',
  Unreachable = 'unreachableNode',
  Loading = 'loading',
}

export const ReachableNode = io.type({
  type: io.literal(NodeType.Reachable),
  addr: io.string,
  details: io.type({
    addrs: io.string,
    nodeId: io.string,
    displayName: io.string,
    startedIso: io.string,
    startedUnix: io.Int,
    version: io.string,
    settings: io.UnknownRecord,
    settingsSchema: io.UnknownRecord,
    swarmState: NodeSwarmState,
    offsets: io.union([io.null, SwarmOffsets]),
  }),
})
export type ReachableNode = io.TypeOf<typeof ReachableNode>
const UnauthorizedNode = io.type({
  type: io.literal(NodeType.Unauthorized),
  addr: io.string,
})
const UnreachableNode = io.type({
  type: io.literal(NodeType.Unreachable),
  addr: io.string,
})
const LoadingNode = io.type({
  type: io.literal(NodeType.Loading),
  addr: io.string,
})

export const Node = io.union([ReachableNode, UnauthorizedNode, UnreachableNode, LoadingNode])
export type Node = io.TypeOf<typeof Node>

// Helpers
const RequestWithAddr = io.type({ addr: io.string })
const RequestWithAddrs = io.type({ addrs: io.array(io.string) })
const EmptyRequest = io.type({})
const Void = io.void

// Get node details
export const GetNodeDetailsRequest = RequestWithAddr
export type GetNodeDetailsRequest = io.TypeOf<typeof GetNodeDetailsRequest>

export const GetNodeDetailsResponse = io.array(Node)
export type GetNodeDetailsResponse = io.TypeOf<typeof GetNodeDetailsResponse>

// Get nodes details
export const GetNodesDetailsRequest = RequestWithAddrs
export type GetNodesDetailsRequest = io.TypeOf<typeof GetNodesDetailsRequest>

export const GetNodesDetailsResponse = io.array(Node)
export type GetNodesDetailsResponse = io.TypeOf<typeof GetNodesDetailsResponse>

// Get node settings
export const GetSettingsRequest = RequestWithAddr
export type GetSettingsRequest = io.TypeOf<typeof GetSettingsRequest>

export const GetSettingsResponse = io.type({
  settings: io.unknown,
})
export type GetSettingsResponse = io.TypeOf<typeof GetSettingsResponse>

// Set node settings
export const SetSettingsRequest = io.intersection([
  RequestWithAddr,
  io.type({
    settings: io.unknown,
  }),
])
export type SetSettingsRequest = io.TypeOf<typeof SetSettingsRequest>

export const SetSettingsResponse = Void
export type SetSettingsResponse = io.TypeOf<typeof SetSettingsResponse>

// Create a user key pair
export const CreateUserKeyPairRequest = io.type({
  privateKeyPath: io.union([io.string, io.null]),
})
export type CreateUserKeyPairRequest = io.TypeOf<typeof CreateUserKeyPairRequest>

export const CreateUserKeyPairResponse = io.type({
  privateKeyPath: io.string,
  publicKeyPath: io.string,
  publicKey: io.string,
})
export type CreateUserKeyPairResponse = io.TypeOf<typeof CreateUserKeyPairResponse>

// Generate a swarm key
export const GenerateSwarmKeyRequest = EmptyRequest
export type GenerateSwarmKeyRequest = io.TypeOf<typeof GenerateSwarmKeyRequest>

export const GenerateSwarmKeyResponse = io.type({
  swarmKey: io.string,
})
export type GenerateSwarmKeyResponse = io.TypeOf<typeof GenerateSwarmKeyResponse>

// Create signed app manifest
export const SignAppManifestRequest = io.type({
  pathToManifest: io.string,
  pathToCertificate: io.string,
})

export type SignAppManifestRequest = io.TypeOf<typeof SignAppManifestRequest>

export const SignAppManifestResponse = io.type({})
export type SignAppManifestResponse = io.TypeOf<typeof SignAppManifestResponse>

// Shutdown node
export const ShutdownNodeRequest = RequestWithAddr
export type ShutdownNodeRequest = io.TypeOf<typeof ShutdownNodeRequest>

export const ShutdownNodeResponse = Void
export type ShutdownNodeResponse = io.TypeOf<typeof ShutdownNodeResponse>
