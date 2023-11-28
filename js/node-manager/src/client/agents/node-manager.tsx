import * as E from 'fp-ts/Either'
import { OffsetInfo } from '../offsets'
import { Obs, Vaettir, VaettirReact } from 'vaettir-react'
import { ObsValcon } from 'systemic-ts-utils/valcon'
import { IterCell } from 'systemic-ts-utils/iter-cell'
import { NodeType, ReachableNodeUi, UiNode } from '../../common/types/nodes'
import { safeErrorToStr, sleep } from '../../common/util'
import { DEFAULT_TIMEOUT_SEC } from '../../common/consts'
import { NodeInfo, NodeInfoProvider } from './node-info-provider'
import {
  PublishRequest,
  PublishResponse,
  QueryResponse,
  TopicDeleteResponse,
  TopicLsResponse,
} from '../../common/types'
import * as util from '../util'
import { Favorite, FavoriteParams } from './favorite-manager'
export { Favorite, FavoriteParams }

const POLLING_INTERVAL_MS = 1000

// Agent definition
// ================

export const NodeManagerAgentContext = VaettirReact.Context.make<NodeManagerAgent>()

export type NodeManagerAgent = ReturnType<typeof NodeManagerAgent>

export const NodeManagerAgent = ({
  allowedToWorkRef,
  timeoutRef,
  favoriteParams,
}: {
  allowedToWorkRef: ObsValcon<boolean>
  timeoutRef: ObsValcon<number | null>
  favoriteParams: FavoriteParams
}) =>
  Vaettir.build()
    .channels((channels) => ({
      ...channels,
      onNodeDisconnect: Obs.make<string>(),
      onNodeInfoChange: Obs.make<string>(),
    }))
    .api(({ channels, isDestroyed, onDestroy }) => {
      const favorites = Favorite(favoriteParams)

      const data = Object.seal({
        trackedAddresses: IterCell.make(new Set()),
        disconnectionPeriodExpiry: IterCell.make(new Map()),
        connections: IterCell.make(new Map()),
        nodeDetailsProviders: IterCell.make(new Map()),
        offsets: null as OffsetInfo | null,
      } as Internals)

      const memoTrackedAddrs = IterCell.Lazy.make(data.trackedAddresses, (trackedAddrs) =>
        Array.from(trackedAddrs),
      )

      const memoReachableUINodes = IterCell.Lazy.make(data.nodeDetailsProviders, (providers) =>
        Array.from(providers.values())
          .map((provider) => provider.api.getAsReachableNodeUi())
          .filter((info): info is ReachableNodeUi => info !== null),
      )

      const memoConnectedNodes = IterCell.Lazy.make(data.connections, (connections) =>
        Array.from(connections.entries())
          .map(([addr, connection]) => {
            if (!E.isRight(connection)) return null
            return { addr, peer: connection.right.peer }
          })
          .filter((x): x is { addr: string; peer: string } => x !== null),
      )

      const memoNodeAsUINode = IterCell.Lazy.make(data.trackedAddresses, (addresses) =>
        IterCell.Lazy.make(data.disconnectionPeriodExpiry, (disconnectionsExpiry) =>
          IterCell.Lazy.make(data.connections, (connections) =>
            IterCell.Lazy.make(data.nodeDetailsProviders, (infoproviders) => {
              const alltracked = Array.from(addresses)

              return alltracked.map(
                (addr): UiNode =>
                  intoUiNode(
                    addr,
                    disconnectionsExpiry.get(addr) || null,
                    connections.get(addr) || null,
                    infoproviders.get(addr)?.api.get() || null,
                  ),
              )
            }),
          ),
        ),
      )

      const unsubs = [
        // After favorites is initialized: populate tracked addresses with favorites
        favorites.channels.initialized.sub(() => {
          data.trackedAddresses.mutate((tracked) =>
            favorites.api.getFavorites().forEach((favorite) => tracked.add(favorite)),
          )
          channels.change.emit()
        }),
        // Propagate all change events from favorites
        favorites.channels.change.sub(channels.change.emit),
        // On node disconnection, dissolve "connection"
        channels.onNodeDisconnect.sub((addr) => {
          data.connections.mutate((connection) => {
            console.log('disconnected from:', addr)
            connection.delete(addr)
          })
          data.disconnectionPeriodExpiry.mutate((reconnect) => {
            // 5 seconds delay before reconnecting
            reconnect.set(addr, new Date(Date.now() + 5000))
          })
          regulateRemovals(data, channels.change.emit)
        }),
        // On node info change, switch nodedetailsprovider to invalidate "memoNodesAsUINode"
        // And refresh data.offsets
        channels.onNodeInfoChange.sub((addr) => {
          data.nodeDetailsProviders.mutate((x) => x)
          data.offsets = OffsetInfo.of(memoReachableUINodes.call())
          channels.change.emit()
        }),
      ]

      const purge = () => {
        console.log('node manager purged')
        data.trackedAddresses.mutate((x) => x.clear())
        regulateRemovals(data, channels.change.emit)
      }

      onDestroy(() => {
        unsubs.map((unsub) => unsub())
      })

      // Recurring task
      ;(async () => {
        while (!isDestroyed()) {
          const getTimeoutSec = timeoutRef.get() || DEFAULT_TIMEOUT_SEC

          if (allowedToWorkRef.get()) {
            regulateRemovals(data, channels.change.emit)
            await regulateNewConnections(data, getTimeoutSec, timeoutRef, {
              onChange: channels.change.emit,
              onNodeInfoChange: channels.onNodeInfoChange.emit,
              onNodeDisconnect: channels.onNodeDisconnect.emit,
            })
          }

          await sleep(POLLING_INTERVAL_MS)
        }
        purge()
      })()

      return {
        ...makeExtendedControl(data),

        getOffsets: () => data.offsets,
        getAllTrackedAddrs: memoTrackedAddrs.call,
        getReachableUiNodes: memoReachableUINodes.call,
        getConnectedNodes: memoConnectedNodes.call,
        getNodesAsUiNode: () => memoNodeAsUINode.call().call().call().call(),
        getNodeAsUiNode: (addr: string) => {
          const isTracking = data.trackedAddresses.access().has(addr)
          return isTracking
            ? intoUiNode(
                addr,
                data.disconnectionPeriodExpiry.access().get(addr) || null,
                data.connections.access().get(addr) || null,
                data.nodeDetailsProviders.access().get(addr)?.api.get() || null,
              )
            : null
        },
        favorites: favorites.api,

        addNodes: (addrs: string[]) => {
          data.trackedAddresses.mutate((trackedAddrs) => {
            addrs.forEach((addr) => trackedAddrs.add(addr))
          })
          channels.change.emit()
        },
        removeNodes: (addrs: string[]) => {
          data.trackedAddresses.mutate((trackedAddrs) => {
            addrs.forEach((addr) => trackedAddrs.delete(addr))
          })
          channels.change.emit()
        },
      }
    })
    .finish()

// Implementation Details
// ======================

const intoUiNode = (
  addr: string,
  disconnectionPeriodExpiry: Date | null,
  connection: ConnectionResult | null,
  nodeInfo: NodeInfo | null,
): UiNode => {
  if (disconnectionPeriodExpiry) return { type: NodeType.Disconnected, addr }
  if (!connection) return { type: NodeType.Connecting, addr, prevError: null }
  if (E.isLeft(connection)) return { type: NodeType.Connecting, addr, prevError: connection.left }
  const { peer } = connection.right

  if (!nodeInfo) return { type: NodeType.Connected, addr, peer }
  if (nodeInfo.type === NodeType.Reachable) {
    return { ...nodeInfo, addr, timeouts: nodeInfo.timeouts }
  }
  return { ...nodeInfo, addr }
}

/**
 * Internals, regulateRemovals, and regulateNewConnections are designed
 * so that `regulateRemovals` can be called as an interruptions (concurrently by many parties)
 */
type Internals = {
  /**
   * User-indicated addresses that needs to be tracked
   * IMMUTABLE, must be comparable with nodes === nodes
   */
  trackedAddresses: IterCell<Set<string>>
  /**
   * After a disconnection, reconnectionAvailabilityTime records the timetstamp
   * which an address will be for reconnection
   */
  disconnectionPeriodExpiry: IterCell<Map<string, Date>>
  /**
   * "Connection" to nodes, containing either peer info or error
   * IMMUTABLE, must be comparable with nodes === nodes
   */
  connections: IterCell<Map<string, ConnectionResult>>
  /**
   * Contains agents providing node details asynchonously
   */
  nodeDetailsProviders: IterCell<Map<string, NodeInfoProvider>>
  offsets: null | OffsetInfo
}

type ConnectionResult = E.Either<string, { peer: string }>

/**
 * Remove nodes whose addresses are untracked.
 *
 * Multiple regulateRemovals can run concurrently.
 */
const regulateRemovals = (data: Internals, onChange: () => unknown) => {
  const disconnectibles = getDisconnectibles(data)
  const removableNodeDetailsProviders = getRemoveableDetailsProviders(data)
  const removableDisconnectionPeriod = getRemovableDisconnectionPeriod(data)

  if (disconnectibles.length > 0) {
    data.connections.mutate((nodes) => {
      disconnectibles.forEach((addr) => {
        nodes.delete(addr)

        console.log('disconnect from:', addr)
      })
    })
  }

  if (removableNodeDetailsProviders.length > 0) {
    data.nodeDetailsProviders.mutate((nodeInfoProviders) => {
      removableNodeDetailsProviders.forEach((addr) => {
        nodeInfoProviders.get(addr)?.destroy()
        nodeInfoProviders.delete(addr)

        console.log('stop tracking node:', addr)
      })
    })
  }

  if (removableDisconnectionPeriod.length > 0) {
    data.disconnectionPeriodExpiry.mutate((map) => {
      removableDisconnectionPeriod.forEach((addr) => {
        map.delete(addr)

        console.log('disconnected node available for reconnection:', addr)
      })
    })
  }

  if (
    disconnectibles.length +
      removableNodeDetailsProviders.length +
      removableDisconnectionPeriod.length >
    0
  ) {
    onChange()
  }
}

const isInDisconnectionPeriod = (data: Internals, addr: string, now = new Date()) => {
  const disconnectionExpiry = data.disconnectionPeriodExpiry.access().get(addr)
  if (!disconnectionExpiry) return false
  return disconnectionExpiry.getTime() > now.getTime()
}

/**
 * There must be only one instance of `regulateNewConnections` running.
 *
 * regulate `connections` according to the list of `trackedAddrs`
 * regulate `nodeDetailsProviders` according to `connections`
 */
const regulateNewConnections = async (
  data: Internals,
  timeout: number,
  timeoutRef: ObsValcon<number | null>,
  events: {
    onChange: () => unknown
    onNodeInfoChange: (addr: string) => unknown
    onNodeDisconnect: (addr: string) => unknown
  },
) => {
  const now = new Date()
  // Connect to newly tracked nodes
  const connectibles = Array.from(data.trackedAddresses.access()).filter((addr) => {
    const connection = data.connections.access().get(addr)
    const inDisconnectionPeriod = isInDisconnectionPeriod(data, addr, now)
    return !inDisconnectionPeriod && (!connection || E.isLeft(connection))
  })

  if (connectibles.length > 0) {
    const connections = await attemptConnections(connectibles, timeout)
    data.connections.mutate((nodes) => {
      Array.from(connections).map(({ addr, res }) => {
        nodes.set(addr, res)

        if (E.isRight(res)) {
          console.log('connected to', addr, res.right.peer)
        } else {
          console.warn('connect error', addr, res.left)
        }
      })
    })
    events.onChange()
  }

  // Add nodeinfoproviders
  const newProviderInfo = Array.from(data.connections.access().entries())
    .map(([addr, nodePeerInfo]) => {
      const existingProvider = data.nodeDetailsProviders.access().get(addr)
      if (existingProvider !== undefined) return null
      if (E.isLeft(nodePeerInfo)) return null
      return { addr, peer: nodePeerInfo.right.peer } as const
    })
    .filter((x): x is { addr: string; peer: string } => {
      // prefer !!x because TypeScript does not correctly determine this assertion (x !== undefined is allowed, which will cause a bug)
      return !!x
    })

  if (newProviderInfo.length > 0) {
    data.nodeDetailsProviders.mutate((providermap) => {
      newProviderInfo.map(({ addr, peer }) => {
        console.log('start tracking node:', addr, peer)

        providermap.set(
          addr,
          NodeInfoProvider({
            timeoutRef,
            addr,
            peer,
            emitNodeInfoChange: () => events.onNodeInfoChange(addr),
            emitDisconnect: () => events.onNodeDisconnect(addr),
          }),
        )
      })
    })

    events.onChange()
  }
}

const getDisconnectibles = (data: Internals) => {
  const connections = data.connections.access()
  const addresses = data.connections.access()
  return Array.from(connections.keys()).filter((addr) => !addresses.has(addr))
}

const getRemoveableDetailsProviders = (data: Internals) => {
  const connections = data.connections.access()
  const providers = data.nodeDetailsProviders.access()
  return Array.from(providers.keys()).filter((addr) => !connections.has(addr))
}

const getRemovableDisconnectionPeriod = (data: Internals) => {
  const now = new Date().getTime()
  const disconnectionPeriodExpiry = data.disconnectionPeriodExpiry.access()
  return Array.from(disconnectionPeriodExpiry.entries())
    .filter(([_, expiry]) => expiry.getTime() <= now)
    .map(([addr, _]) => addr)
}

const attemptConnections = (
  addresses: string[],
  timeout: number,
): Promise<
  {
    addr: string
    res: ConnectionResult
  }[]
> =>
  Promise.all(
    addresses.map((addr) =>
      util
        .connect({ addr, timeout })
        .then(({ peer }) => ({ addr, res: E.right({ peer }) }))
        .catch((err) => ({ addr, res: E.left(safeErrorToStr(err)) })),
    ),
  )

// Extended Control
// ================

type ExtendedControl = {
  shutdownNode: (addr: string) => Promise<void>
  setSettings: (addr: string, settings: object, scope: string[]) => Promise<void>
  publish: (args: { addr: string; events: PublishRequest['events'] }) => Promise<PublishResponse>
  query: (args: { addr: string; query: string }) => Promise<QueryResponse>
  getTopicList: (addr: string) => Promise<TopicLsResponse>
  deleteTopic: (addr: string, topic: string) => Promise<TopicDeleteResponse>
}

export const makeExtendedControl = (data: Internals): ExtendedControl => {
  const withPeer = <Fn extends (peer: string) => Promise<R>, R>(
    addr: string,
    fn: Fn,
  ): Promise<R> => {
    const peerData = data.connections.access().get(addr)
    if (peerData && E.isRight(peerData)) {
      return fn(peerData.right.peer)
    } else {
      return Promise.reject(`not connected to ${addr}`)
    }
  }
  return {
    shutdownNode: (addr) => withPeer(addr, (peer) => util.shutdownNode({ peer })),
    setSettings: (addr, settings, scope) =>
      withPeer(addr, (peer) => util.setSettings({ peer, settings, scope })),
    query: ({ addr, query }) =>
      withPeer(addr, (peer) =>
        util.query({
          peer,
          query,
        }),
      ),
    publish: ({ addr, events }) =>
      withPeer(addr, (peer) =>
        util.publish({
          peer,
          events,
        }),
      ),
    getTopicList: (addr) => withPeer(addr, (peer) => util.getTopicList({ peer })),
    deleteTopic: (addr, topic) => withPeer(addr, (peer) => util.deleteTopic({ peer, topic })),
  }
}
