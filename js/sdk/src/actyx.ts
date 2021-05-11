/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { EventFns, TestEventFns } from './event-fns'
import { SnapshotStore } from './snapshotStore'
import { ActyxOpts, ActyxTestOpts } from './types'
import { EventFnsFromEventStoreV2, EventStoreV2, makeWsMultiplexerV2 } from './v2'

/** Access all sorts of functionality related to the Actyx system! @public */
export type Actyx = EventFns & {
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
  /** For `TestActyx` instances, this method does nothing; it’s just there so all normal `Actyx` functions are provided. @public */
  dispose: () => void

  /** Underlying snapshotstore, only for testing snapshot interactions. FIXME: Define proper public snapshot API on `Actyx`. @alpha */
  // snapshotStore: SnapshotStore
}

/** Function for creating `Actyx` instances. @public */
export const Actyx = {
  /** Create an `Actyx` instance that talk to a running `Actyx` system. @public */
  of: async (_manifest: unknown, opts: ActyxOpts = {}): Promise<Actyx> => {
    const [ws, nodeId] = await makeWsMultiplexerV2(opts)
    const eventStore = EventStoreV2.ws(ws, nodeId)
    // No snapshotstore impl available for V2 prod
    const fns = EventFnsFromEventStoreV2(eventStore, SnapshotStore.noop)

    return {
      ...fns,
      dispose: () => ws.close(),
    }
  },

  /** Create an `Actyx` instance that mocks all functionality in TS. Useful for unit-tests. @public */
  test: (opts: ActyxTestOpts = {}): TestActyx => {
    const store = EventStoreV2.test(opts.nodeId, opts.eventChunkSize)
    const snaps = SnapshotStore.inMem()
    const fns = EventFnsFromEventStoreV2(store, snaps)
    return {
      ...fns,
      directlyPushEvents: store.directlyPushEvents,
      dispose: () => {
        // Nothing to do, probably.
      },
      // snapshotStore: snaps,
    }
  },
}
