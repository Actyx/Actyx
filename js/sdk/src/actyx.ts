/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { EventFns, TestEventFns } from './event-fns'
import { EventFnsFromEventStoreV2, EventStoreV2 } from './internal_common'
import { log } from './internal_common/log'
import { getInfo, invalidNodeInfo, NodeInfo } from './node-info'
import { SnapshotStore } from './snapshotStore'
import { ActyxOpts, ActyxTestOpts, AppId, AppManifest, NodeId } from './types'
import { mkV1eventStore } from './v1'
import { makeWsMultiplexerV2, v2getNodeId, WebsocketEventStoreV2 } from './v2'
import { BlobSnapshotStore } from './v2/blobSnapshotStore'
import { getApiLocation, getToken, v2WaitForSwarmSync } from './v2/utils'

/** Access all sorts of functionality related to the Actyx system!
 * @public */
export type Actyx = EventFns & {
  /** Id of the Actyx node this interface is connected to. */
  readonly nodeId: NodeId

  /** Fish snapshot store for the Pond
   * @beta */
  readonly snapshotStore: SnapshotStore

  /** Dispose of this Actyx connector, cancelling all ongoing subscriptions and freeing all underlying ressources.
   * @public */
  dispose: () => void

  /** Wait for the connected node to be in sync with the swarm. This is on a
   * best-effort basis and waits at most 30 seconds after the point in time the
   * node has been started. This can be used in order to reduce stale state
   * inside the application when started together with an Actyx node.
   * @public */
  waitForSync: () => Promise<void>

  /**
   * Obtain information on the Actyx node. In order to save some cycles, and because the information
   * doesn’t change all that quickly, please provide a time parameter that matches your app’s
   * freshness requirements — for human consumption a couple hundred milliseconds is good enough.
   *
   * The underlying API endpoint has been added in Actyx 2.5.0, earlier versions report dummy data.
   *
   * Starting with Actyx 2.15.0 the returned data also contains the swarm status as seen by this node.
   *
   * @public
   */
  nodeInfo: (maxAgeMillis: number) => Promise<NodeInfo>
}

/**
 * An instance of `Actyx` that is not talking to any Actyx instance, but mocks all functionality within TypeScript.
 * Very useful for unit-testing.
 *
 * @public
 */
export type TestActyx = TestEventFns & {
  /** Prented id of the underlying Actyx instance that actually is just simulated. */
  readonly nodeId: NodeId

  /** Fish snapshot store for the Pond
   * @beta */
  readonly snapshotStore: SnapshotStore

  /** For `TestActyx` instances, this method does nothing; it’s just there so all normal `Actyx` functions are provided.
   * @public */
  dispose: () => void

  waitForSync: () => Promise<void>
  /** Underlying snapshotstore, only for testing snapshot interactions. FIXME: Define proper public snapshot API on `Actyx`.
   * @alpha */
  // snapshotStore: SnapshotStore
}

const createV2 = async (manifest: AppManifest, opts: ActyxOpts, nodeId: string): Promise<Actyx> => {
  const token = await getToken(opts, manifest)
  const [ws, tok] = await makeWsMultiplexerV2(opts, token, manifest)
  const eventStore = new WebsocketEventStoreV2(ws, AppId.of(manifest.appId), () => tok[1])
  const snapshotStore = new BlobSnapshotStore(
    'http://' + getApiLocation(opts.actyxHost, opts.actyxPort),
    () => tok[0],
    () => tok[1],
    100_000_000,
  )
  const fns = EventFnsFromEventStoreV2(nodeId, eventStore, snapshotStore, () => tok[1])

  const waitForSync = async () => v2WaitForSwarmSync(opts, token, fns.offsets)
  const getNodeInfo = getInfo(opts)

  return {
    ...fns,
    snapshotStore,
    nodeId,
    dispose: () => ws.close(),
    waitForSync,
    nodeInfo: (maxAgeMillis) => getNodeInfo(tok[0], maxAgeMillis),
  }
}

const createV1 = async (opts: ActyxOpts): Promise<Actyx> => {
  const { eventStore, sourceId, close, snapshotStore } = await mkV1eventStore(opts)

  const fns = EventFnsFromEventStoreV2(sourceId, eventStore, snapshotStore, () => '1.0.0')

  return {
    ...fns,
    snapshotStore,
    nodeId: sourceId,
    dispose: () => close(),
    waitForSync: () => Promise.resolve(),
    nodeInfo: () => Promise.resolve(invalidNodeInfo),
  }
}

/** Function for creating `Actyx` instances.
 * @public */
export const Actyx = {
  /** Create an `Actyx` instance that talks to a running `Actyx` system.
   * @public */
  of: async (manifest: AppManifest, opts: ActyxOpts = {}): Promise<Actyx> => {
    const nodeId = await v2getNodeId(opts)
    log.actyx.debug('NodeId call returned:', nodeId)

    if (!nodeId) {
      // Try connecting to v1 if we failed to retrieve a v2 node id
      // (Note that if the port is completely unreachable, v2getNodeId will throw an exception and we don’t get here.)
      log.actyx.debug('NodeId was null, trying to reach V1 backend...')
      return createV1(opts)
    }

    log.actyx.debug(
      'Detected V2 is running, trying to authorize with manifest',
      JSON.stringify(manifest),
    )
    return createV2(manifest, opts, nodeId)
  },

  /**
   * Create an `Actyx` instance that mocks all APIs within TypeScript. Useful for unit-tests.
   * Will not talk to other nodes, but rather that can be simulated via `directlyPushEvents`.
   * @public
   */
  test: (opts: ActyxTestOpts = {}): TestActyx => {
    const store = EventStoreV2.test(opts.nodeId, opts.timeInjector)
    const snaps = SnapshotStore.inMem()
    const nodeId = opts.nodeId || NodeId.of('TESTNODEID')

    const fns = EventFnsFromEventStoreV2(nodeId, store, snaps, () => '2.0.0')
    return {
      ...fns,
      snapshotStore: snaps,
      nodeId,
      directlyPushEvents: store.directlyPushEvents,
      dispose: () => {
        store.close()
      },
      waitForSync: async () => {
        /* noop */
      },
      // snapshotStore: snaps,
    }
  },
}
