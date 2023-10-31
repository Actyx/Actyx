import { Failure, PingStats } from 'common/types'
import * as io from 'io-ts'

export const enum NodeType {
  Reachable = 'reachableNode',
  Unauthorized = 'unauthorizedNode',
  Unreachable = 'unreachableNode',
  Fresh = 'fresh',
  Disconnected = 'disconnectedNode',
  Connecting = 'connecting',
  Connected = 'connected',
}

export const PeerInfo = io.type({
  protocolVersion: io.union([io.null, io.string]),
  agentVersion: io.union([io.null, io.string]),
  protocols: io.array(io.string),
  listeners: io.array(io.string),
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
})
export type DisconnectedNode = io.TypeOf<typeof DisconnectedNode>

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

export const GetNodeDetailsResponse = Node
export type GetNodeDetailsResponse = io.TypeOf<typeof GetNodeDetailsResponse>
