// add nodes
// rem nodes
// restart nodes

import * as E from 'fp-ts/Either'
import { OffsetInfo } from '../offsets'
import { Obs, Serv } from '../util/serv'
import { GetNodeDetailsResponse, NodeType, ReachableNodeUi, UiNode } from '../../common/types/nodes'
import { safeErrorToStr, sleep } from '../../common/util'
import { DEFAULT_TIMEOUT_SEC } from '../../common/consts'
import { NodeInfoProvider } from './node-info-provider'
import {
  PublishRequest,
  PublishResponse,
  QueryResponse,
  TopicDeleteResponse,
  TopicLsResponse,
} from '../../common/types'
import * as util from '../util'
import { ServReact } from '../util/serv-react'
import { ObsValcon } from '../util/valcon'

const POLLING_INTERVAL_MS = 1000

// Agent definition

// TODO: Implement Favorited Addresses
// TODO: Implement using timeout from preferences

export const NodeManagerAgentContext = ServReact.Context.make<NodeManagerAgent>()

export type NodeManagerAgent = ReturnType<typeof NodeManagerAgent>

export const NodeManagerAgent = (allowedToWorkRef: ObsValcon<boolean>) =>
  Serv.build()
    .channels((channels) => ({
      ...channels,
      onNodeDisconnect: Obs.make<string>(),
      onNodeInfoChange: Obs.make<string>(),
    }))
    .api(({ channels, isDestroyed, onDestroy }) => {
      const data: Internals = Object.seal({
        trackedAddresses: ImmutableContainer(new Set()),
        connections: ImmutableContainer(new Map()),
        nodeDetailsProviders: ImmutableContainer(new Map()),
        offsets: null as OffsetInfo | null,
      })

      const getAllTrackedAddrs = ImmutableLazyCalc(data.trackedAddresses, (trackedAddrs) =>
        Array.from(trackedAddrs),
      )

      const getReachableUiNodes = ImmutableLazyCalc(data.nodeDetailsProviders, (providers) =>
        Array.from(providers.values())
          .map((provider) => provider.api.getAsReachableNodeUi())
          .filter((info): info is ReachableNodeUi => info !== null),
      )

      const getConnectedNodes = ImmutableLazyCalc(data.connections, (connections) =>
        Array.from(connections.entries())
          .map(([addr, connection]) => {
            if (!E.isRight(connection)) return null
            return { addr, peer: connection.right.peer }
          })
          .filter((x): x is { addr: string; peer: string } => x !== null),
      )

      const getNodesAsUiNode = ImmutableLazyCalc(data.trackedAddresses, (addresses) =>
        ImmutableLazyCalc(data.connections, (connections) =>
          ImmutableLazyCalc(data.nodeDetailsProviders, (infoproviders) => {
            const alltracked = Array.from(addresses)

            return alltracked.map(
              (addr): UiNode =>
                intoUiNode(
                  addr,
                  connections.get(addr) || null,
                  infoproviders.get(addr)?.api.get() || null,
                ),
            )
          }),
        ),
      )

      const unsubs = [
        channels.onNodeDisconnect.sub((addr) => {
          data.connections.mutate((connection) => {
            console.log('disconnected from:', addr)
            connection.delete(addr)
          })
          regulateRemovals(data, channels.change.emit)
        }),
        channels.onNodeInfoChange.sub((addr) => {
          // swap node detail providers - triggering cache-invalidation to `getNodesAsUiNode`
          data.nodeDetailsProviders.mutate((x) => x)
          // refresh offsets everytime there's a change in NodeInfoChange
          data.offsets = OffsetInfo.of(getReachableUiNodes())
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
          // const getTimeoutSec =
          //   (store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout) ||
          //   DEFAULT_TIMEOUT_SEC

          if (allowedToWorkRef.get()) {
            const getTimeoutSec = DEFAULT_TIMEOUT_SEC
            regulateRemovals(data, channels.change.emit)
            await regulateNewConnections(data, getTimeoutSec, {
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
        getAllTrackedAddrs,
        getReachableUiNodes,
        getConnectedNodes,
        getNodesAsUiNode: () => getNodesAsUiNode()()(),
        getNodeAsUiNode: (addr: string) => {
          const isTracking = data.trackedAddresses.access().has(addr)
          return isTracking
            ? intoUiNode(
                addr,
                data.connections.access().get(addr) || null,
                data.nodeDetailsProviders.access().get(addr)?.api.get() || null,
              )
            : null
        },

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
        control: (addr: string): ExtendedControl => null as any,
      }
    })
    .finish()

// Implementation Details
// ======================

type ImmutableContainer<T extends Set<any> | Map<any, any>> = {
  access: () => Readonly<T>
  mutate: <R>(fn: (set: T) => R) => R
}

const ImmutableContainer = <T extends Set<any> | Map<any, any>>(t: T) => {
  const proto: any = t instanceof Set ? Set : Map
  let inner: T = new proto(t) as T
  const self: ImmutableContainer<T> = {
    access: () => inner,
    mutate: (fn) => {
      inner = new proto(inner)
      return fn(inner)
    },
  }
  return self
}

const ImmutableLazyCalcUninit: unique symbol = Symbol()
const ImmutableLazyCalc = <T extends Set<any> | Map<any, any>, R extends any>(
  immutable: ImmutableContainer<T>,
  fn: (t: Readonly<T>) => R,
): (() => R) => {
  let lastContainerState = immutable.access()
  let cache = ImmutableLazyCalcUninit as typeof ImmutableLazyCalcUninit | R

  return () => {
    if (immutable.access() !== lastContainerState) {
      lastContainerState = immutable.access()
      cache = fn(immutable.access())
    }
    const ret = cache !== ImmutableLazyCalcUninit ? cache : (fn(immutable.access()) as R)
    cache = ret
    return ret
  }
}

const intoUiNode = (
  addr: string,
  connection: ConnectionResult | null,
  nodeInfo: GetNodeDetailsResponse | null,
): UiNode => {
  if (!connection) return { type: NodeType.Connecting, addr, prevError: null }

  if (E.isLeft(connection)) return { type: NodeType.Connecting, addr, prevError: connection.left }
  const { peer } = connection.right

  if (!nodeInfo) return { type: NodeType.Connected, addr, peer }
  if (nodeInfo.type === NodeType.Reachable) return { ...nodeInfo, addr, timeouts: 0 }
  return { ...nodeInfo, addr }
}

type Internals = {
  /**
   * User-indicated addresses that needs to be tracked
   * IMMUTABLE, must be comparable with nodes === nodes
   */
  trackedAddresses: ImmutableContainer<Set<string>>
  /**
   * "Connection" to nodes, containing either peer info or error
   * IMMUTABLE, must be comparable with nodes === nodes
   */
  connections: ImmutableContainer<Map<string, ConnectionResult>>
  /**
   * Contains agents providing node details asynchonously
   */
  nodeDetailsProviders: ImmutableContainer<Map<string, NodeInfoProvider>>
  offsets: null | OffsetInfo
}

type ConnectionResult = E.Either<string, { peer: string }>

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

/**
 * Remove nodes whose addresses are untracked.
 */
const regulateRemovals = (data: Internals, onChange: () => unknown) => {
  const disconnectibles = getDisconnectibles(data)
  const removableNodeDetailsProviders = getRemoveableDetailsProviders(data)

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

  if (disconnectibles.length + removableNodeDetailsProviders.length > 0) {
    onChange()
  }
}

/**
 * When a tracked address does not have
 */
const regulateNewConnections = async (
  data: Internals,
  timeout: number,
  events: {
    onChange: () => unknown
    onNodeInfoChange: (addr: string) => unknown
    onNodeDisconnect: (addr: string) => unknown
  },
) => {
  // Connect to newly tracked nodes
  const connectibles = Array.from(data.trackedAddresses.access()).filter((addr) => {
    const connection = data.connections.access().get(addr)
    console.log('connectible addr', connection)
    return !connection || E.isLeft(connection)
  })

  if (connectibles.length > 0) {
    const connections = await attemptConnections(connectibles, timeout)
    data.connections.mutate((nodes) => {
      Array.from(connections).map(({ addr, res }) => {
        nodes.set(addr, res)

        if (E.isRight(res)) {
          console.log('connected to', addr, res.right.peer)
        } else {
          console.log('connect error', addr, res.left)
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
      sleep(Math.round(Math.random() * 0)).then(() =>
        util
          .connect({ addr, timeout })
          .then(({ peer }) => ({ addr, res: E.right({ peer }) }))
          .catch((err) => ({ addr, res: E.left(safeErrorToStr(err)) })),
      ),
    ),
  )

// Implementation Details
// ======================

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
