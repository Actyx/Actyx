/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import { ord } from 'fp-ts'
import { ordNumber } from 'fp-ts/lib/Ord'
import { Semantics, SubscriptionSet } from '.'
import { Event, EventStore, UnstoredEvent } from './eventstore'
import {
  addAndInvalidateState,
  FishEventStore,
  FishEventStoreImpl,
  FishInfo,
  mergeSortedInto,
} from './fishEventStore'
import { SnapshotStore } from './snapshotStore'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { FishName, SourceId, StateWithProvenance, Timestamp } from './types'

const impl = <S, E>(store: FishEventStore<S, E>): FishEventStoreImpl<S, E> => store as any

type Payload = number
type State = number
const payloadOrder = ord.ordNumber.compare

const processPayload = (state: State, payload: Payload): State => payload + state

const processEvent = (state: State, event: Event): State =>
  processPayload(state, event.payload as Payload)

describe('FishEventStore functions', () => {
  describe('addAndInvalidateState', () => {
    it('should insert events in the right order - no overlap', () => {
      const events1: Payload[] = [0, 1, 2, 3, 4]
      const states: StateWithProvenance<number>[] = [
        { state: 1, psnMap: {} },
        { state: 2, psnMap: {} },
        { state: 3, psnMap: {} },
        { state: 4, psnMap: {} },
        { state: 5, psnMap: {} },
      ]
      const newEvents: ReadonlyArray<Payload> = [5, 6, 7, 8]
      const states1 = states.slice()
      addAndInvalidateState(events1, i => (states1.length = i + 1), newEvents, payloadOrder)
      expect(events1).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8])
      expect(states1).toEqual(states)
    })

    it('should insert events in the right order - interleaved', () => {
      const events1: Payload[] = [0, 2, 4, 6, 8]
      const states: StateWithProvenance<number>[] = [
        { state: 1, psnMap: {} },
        { state: 2, psnMap: {} },
        { state: 3, psnMap: {} },
        { state: 4, psnMap: {} },
        { state: 5, psnMap: {} },
      ]
      const newEvents: ReadonlyArray<Payload> = [1, 3, 5, 7]
      addAndInvalidateState(events1, i => (states.length = i + 1), newEvents, payloadOrder)
      expect(events1).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8])
      expect(states).toEqual([{ state: 1, psnMap: {} }])
    })

    it('should insert events in the right order - duplicates', () => {
      const events1: Payload[] = [0, 2, 4, 6, 8]
      const states: StateWithProvenance<number>[] = [
        { state: 1, psnMap: {} },
        { state: 2, psnMap: {} },
        { state: 3, psnMap: {} },
        { state: 4, psnMap: {} },
        { state: 5, psnMap: {} },
      ]
      const newEvents: ReadonlyArray<Payload> = [3, 4, 5, 6]
      addAndInvalidateState(events1, i => (states.length = i + 1), newEvents, payloadOrder)
      expect(events1).toEqual([0, 2, 3, 4, 5, 6, 8])
      expect(states).toEqual([{ state: 1, psnMap: {} }, { state: 2, psnMap: {} }])
    })
  })

  describe('mergeSortedInto', () => {
    const merge = (l: number[], r: number[]): [number, number[]] => {
      const out = l.slice().concat(...r)
      const h = mergeSortedInto(l, r, out, ordNumber.compare)
      return [h, out]
    }

    it('should sort without overlap', () => {
      expect(merge([1, 2, 3], [4, 5, 6])).toEqual([2, [1, 2, 3, 4, 5, 6]])
    })

    it('should sort with partial overlap', () => {
      expect(merge([1, 2, 3, 4], [4, 5, 6])).toEqual([3, [1, 2, 3, 4, 5, 6]])
    })

    it('should sort with exact overlap', () => {
      expect(merge([1, 2, 3, 4, 5, 6], [4, 5, 6])).toEqual([5, [1, 2, 3, 4, 5, 6]])
    })

    it('should sort with more overlap', () => {
      expect(merge([1, 2, 3, 4, 5, 6], [4, 5])).toEqual([5, [1, 2, 3, 4, 5, 6]])
    })
  })
})

describe('FishEventStore', () => {
  const fish: FishInfo<State> = {
    semantics: Semantics.of('some-fish'),
    fishName: FishName.of('some-name'),
    subscriptionSet: SubscriptionSet.all,
    initialState: () => 0,
    onEvent: processEvent,
    isSemanticSnapshot: undefined,
    snapshotFormat: undefined,
  }
  const source = SourceId.of('sourceA')
  const events = [1, 2, 3, 4, 5]

  const toEnvelope = (payload: Payload, i: number): UnstoredEvent => ({
    semantics: Semantics.of('foo'),
    name: FishName.of('foo'),
    tags: [],
    timestamp: Timestamp.of(i * 1000000),
    payload,
  })

  const mkStore = async (evs: number[]) => {
    const s = EventStore.test(source)
    await s.persistEvents(evs.map(toEnvelope)).toPromise()
    return s
  }

  it('should properly initialize the store', async () => {
    const eventStore = await mkStore(events)
    const present = await eventStore
      .present()
      .take(1)
      .toPromise()
    const store = await FishEventStore.initialize(
      fish,
      eventStore,
      SnapshotStore.noop,
      SnapshotScheduler.create(10),
      present.psns,
    ).toPromise()
    expect(impl(store).events.map(x => x.payload)).toEqual(events)

    const state = await store.currentState().toPromise()
    expect(state.state).toBe(15)

    const state2 = await store.currentState().toPromise()
    expect(state2.state).toBe(15)
  })
})
