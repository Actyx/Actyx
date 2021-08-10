/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
export { EventFnsFromEventStoreV2 } from './event-fns-impl'
export { EventStore as EventStoreV2 } from './eventStore'
export * from './types'

import { Observable } from '../../node_modules/rxjs'
import { EventStore } from './eventStore'
import { mockEventStore } from './mockEventStore'
import { testEventStore } from './testEventStore'

const noopEventStore: EventStore = {
  subscribe: () => Observable.empty(),
  query: () => Observable.empty(),
  queryUnchecked: () => Observable.empty(),
  offsets: () => Promise.resolve({ present: {}, toReplicate: {} }),
  persistEvents: () => Observable.empty(),
}

export const EventStores = {
  noop: noopEventStore,
  mock: mockEventStore,
  test: testEventStore,
}
