import * as io from 'io-ts'

// Disk store

export const StoreData = io.type({
  preferences: io.type({
    favoriteNodeAddrs: io.array(io.string),
    nodeTimeout: io.union([io.undefined, io.number]),
  }),
  analytics: io.type({
    disabled: io.boolean,
    userId: io.string,
  }),
})

export type StoreData = io.TypeOf<typeof StoreData>

// Basics

export const PeerInfo = io.type({
  protocolVersion: io.union([io.null, io.string]),
  agentVersion: io.union([io.null, io.string]),
  protocols: io.array(io.string),
  listeners: io.array(io.string),
})

export const Failure = io.type({
  addr: io.string,
  time: io.string,
  display: io.string,
  details: io.string,
})

export const PingStats = io.type({
  current: io.Int,
  decay3: io.Int,
  decay10: io.Int,
  failures: io.Int,
  failureRate: io.Int,
})

export const Peer = io.type({
  peerId: io.string,
  addrs: io.array(io.string),
  info: io.union([io.undefined, PeerInfo]),
  addrSource: io.union([io.undefined, io.array(io.string)]),
  addrSince: io.union([io.undefined, io.array(io.string)]),
  failures: io.union([io.undefined, io.array(Failure)]),
  pingStats: io.union([io.undefined, io.null, PingStats]),
})

export type Peer = io.TypeOf<typeof Peer>

export const Connection = io.type({
  peerId: io.string,
  addr: io.string,
  since: io.union([io.undefined, io.string]),
  outbound: io.union([io.undefined, io.boolean]),
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
  Fresh = 'fresh',
  Disconnected = 'disconnectedNode',
  Connecting = 'connecting',
  Connected = 'connected',
}

export const ReachableNode = io.type({
  type: io.literal(NodeType.Reachable),
  peer: io.string,
  details: io.type({
    addrs: io.union([io.null, io.string]),
    nodeId: io.string,
    displayName: io.string,
    startedIso: io.string,
    startedUnix: io.Int,
    version: io.string,
    settings: io.UnknownRecord,
    settingsSchema: io.UnknownRecord,
    swarmState: io.union([io.null, NodeSwarmState]),
    offsets: io.union([io.null, SwarmOffsets]),
  }),
})
export type ReachableNode = io.TypeOf<typeof ReachableNode>
export type ReachableNodeUi = ReachableNode & { addr: string; timeouts: number }

const UnauthorizedNode = io.type({
  type: io.literal(NodeType.Unauthorized),
  peer: io.string,
})
type UnauthorizedNode = io.TypeOf<typeof UnauthorizedNode>
const DisconnectedNode = io.type({
  type: io.literal(NodeType.Disconnected),
  peer: io.string,
})
type DisconnectedNode = io.TypeOf<typeof DisconnectedNode>

export const Node = io.union([ReachableNode, UnauthorizedNode, DisconnectedNode])
export type Node = io.TypeOf<typeof Node>

type UnreachableNode = {
  type: NodeType.Unreachable
  addr: string
  error: string
}
type LoadingNode = {
  type: NodeType.Fresh
  addr: string
}
type ConnectingNode = {
  type: NodeType.Connecting
  addr: string
  prevError: string | null
}
type ConnectedNode = {
  type: NodeType.Connected
  addr: string
  peer: string
}

export type UiNode =
  | ReachableNodeUi
  | ((UnauthorizedNode | DisconnectedNode) & { addr: string })
  | UnreachableNode
  | LoadingNode
  | ConnectingNode
  | ConnectedNode

// Helpers
const EmptyRequest = io.type({})
const Void = io.void

// connect to a node
export const ConnectRequest = io.type({
  addr: io.string,
  timeout: io.union([io.null, io.number]),
})
export type ConnectRequest = io.TypeOf<typeof ConnectRequest>
export const ConnectResponse = io.type({
  peer: io.string,
})
export type ConnectResponse = io.TypeOf<typeof ConnectResponse>

// Get node details
export const GetNodeDetailsRequest = io.type({
  peer: io.string,
  timeout: io.union([io.null, io.number]),
})
export type GetNodeDetailsRequest = io.TypeOf<typeof GetNodeDetailsRequest>

export const GetNodeDetailsResponse = Node
export type GetNodeDetailsResponse = io.TypeOf<typeof GetNodeDetailsResponse>

// Set node settings
export const SetSettingsRequest = io.type({
  peer: io.string,
  settings: io.unknown,
})
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

export const SignAppManifestResponse = io.type({
  appId: io.string,
  displayName: io.string,
  version: io.string,
  signature: io.string,
})
export type SignAppManifestResponse = io.TypeOf<typeof SignAppManifestResponse>

// Shutdown node
export const ShutdownNodeRequest = io.type({
  peer: io.string,
})
export type ShutdownNodeRequest = io.TypeOf<typeof ShutdownNodeRequest>

export const ShutdownNodeResponse = Void
export type ShutdownNodeResponse = io.TypeOf<typeof ShutdownNodeResponse>

export const EventResponse = io.type({
  lamport: io.number,
  /// ID of the stream this event belongs to
  stream: io.string,
  /// The event offset within the stream
  offset: io.number,
  /// Timestamp at which the event was emitted
  timestamp: io.number,
  /// Tag attached to the event
  tags: io.array(io.string),
  /// Associated app ID
  appId: io.string,
  /// The actual, app-specific event payload
  payload: io.unknown,
})
export type EventResponse = io.TypeOf<typeof EventResponse>
export const Diagnostic = io.type({
  severity: io.union([io.literal('warning'), io.literal('error')]),
  message: io.string,
})
export type Diagnostic = io.TypeOf<typeof Diagnostic>
export const EventDiagnostic = io.union([EventResponse, Diagnostic])
export type EventDiagnostic = io.TypeOf<typeof EventDiagnostic>

export const QueryRequest = io.type({
  peer: io.string,
  query: io.string,
})
export type QueryRequest = io.TypeOf<typeof QueryRequest>
export const QueryResponse = io.type({
  events: io.union([io.null, io.array(EventDiagnostic)]),
})
export type QueryResponse = io.TypeOf<typeof QueryResponse>
