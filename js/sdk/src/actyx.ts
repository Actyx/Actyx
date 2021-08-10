/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { EventFns, TestEventFns } from './event-fns'
import { EventFnsFromEventStoreV2, EventStores } from './internal_common'
import { log } from './internal_common/log'
import { SnapshotStore } from './snapshotStore'
import { ActyxOpts, ActyxTestOpts, AppId, AppManifest, NodeId } from './types'
import { mkV1eventStore } from './v1'
import { makeWsMultiplexerV2, v2getNodeId, WebsocketEventStoreV2 } from './v2'

/** Access all sorts of functionality related to the Actyx system! @public */
export type Actyx = EventFns & {
  /** Id of the Actyx node this interface is connected to. */
  readonly nodeId: NodeId

  /** Dispose of this Actyx connector, cancelling all ongoing subscriptions and freeing all underlying ressources. @public */
  dispose: () => void
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

  /** For `TestActyx` instances, this method does nothing; it’s just there so all normal `Actyx` functions are provided. @public */
  dispose: () => void

  /** Underlying snapshotstore, only for testing snapshot interactions. FIXME: Define proper public snapshot API on `Actyx`. @alpha */
  // snapshotStore: SnapshotStore
}

const createV2 = async (manifest: AppManifest, opts: ActyxOpts, nodeId: string): Promise<Actyx> => {
  const ws = await makeWsMultiplexerV2(manifest, opts)
  const eventStore = new WebsocketEventStoreV2(ws, AppId.of(manifest.appId))
  // No snapshotstore impl available for V2 prod
  const fns = EventFnsFromEventStoreV2(nodeId, eventStore, SnapshotStore.noop)

  return {
    ...fns,
    nodeId,
    dispose: () => ws.close(),
  }
}

const createV1 = async (opts: ActyxOpts): Promise<Actyx> => {
  const { eventStore, sourceId, close } = await mkV1eventStore(opts)

  const fns = EventFnsFromEventStoreV2(sourceId, eventStore, SnapshotStore.noop)

  return {
    ...fns,
    nodeId: sourceId,
    dispose: () => close(),
  }
}

/** Function for creating `Actyx` instances. @public */
export const Actyx = {
  /** Create an `Actyx` instance that talks to a running `Actyx` system. @public */
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
    const store = EventStores.test(opts.nodeId)
    const snaps = SnapshotStore.inMem()
    const nodeId = opts.nodeId || NodeId.of('TESTNODEID')

    const fns = EventFnsFromEventStoreV2(nodeId, store, snaps)
    return {
      ...fns,
      nodeId,
      directlyPushEvents: store.directlyPushEvents,
      dispose: () => {
        store.close()
      },
      // snapshotStore: snaps,
    }
  },
}
