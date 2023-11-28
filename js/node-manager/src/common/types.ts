import * as io from 'io-ts'

// Disk store

export const StoreData = io.type({
  preferences: io.type({
    favoriteNodeAddrs: io.array(io.string),
    nodeTimeout: io.union([io.undefined, io.number]),
  }),
})

export type StoreData = io.TypeOf<typeof StoreData>

// Basics

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

// Set node settings
export const SetSettingsRequest = io.type({
  peer: io.string,
  settings: io.unknown,
  scope: io.array(io.string),
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

export const PublishResponseKey = io.type({
  lamport: io.number,
  stream: io.string,
  offset: io.number,
  timestamp: io.number,
})
export type PublishResponseKey = io.TypeOf<typeof PublishResponseKey>

export const PublishRequest = io.type({
  peer: io.string,
  events: io.array(
    io.type({
      tags: io.array(io.string),
      payload: io.unknown,
    }),
  ),
})
export type PublishRequest = io.TypeOf<typeof PublishRequest>
export const PublishResponse = io.type({
  data: io.array(PublishResponseKey),
})
export type PublishResponse = io.TypeOf<typeof PublishResponse>

// Topics
export const TopicLsRequest = io.type({ peer: io.string })
export type TopicLsRequest = io.TypeOf<typeof TopicLsRequest>

export const TopicLsResponse = io.type({
  nodeId: io.string,
  activeTopic: io.string,
  topics: io.record(io.string, io.number),
})
export type TopicLsResponse = io.TypeOf<typeof TopicLsResponse>

export const TopicDeleteRequest = io.type({
  peer: io.string,
  topic: io.string,
})
export type TopicDeleteRequest = io.TypeOf<typeof TopicDeleteRequest>

export const TopicDeleteResponse = io.type({
  nodeId: io.string,
  deleted: io.boolean,
})
export type TopicDeleteResponse = io.TypeOf<typeof TopicDeleteResponse>
