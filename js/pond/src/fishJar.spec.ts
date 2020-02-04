/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable } from 'rxjs'
import { EventStore } from './eventstore'
import { Event } from './eventstore/types'
import { CommandExecutor } from './executors/commandExecutor'
import { hydrate } from './fishJar'
import { defaultTimeInjector, makeEventChunk, SendToStore } from './pond'
import { mkNoopPondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { EnvelopeFromStore } from './store/util'
import { Subscription } from './subscription'
import { eventStressFish, eventStressFishBuilder, State } from './testkit/eventStressFish'
import {
  FishName,
  FishType,
  FishTypeImpl,
  InitialState,
  Lamport,
  OnStateChange,
  Psn,
  Semantics,
  Source,
  SourceId,
  Timestamp,
} from './types'
import { noop } from './util'

const fakeObserve: <C, E, P>(fish: FishType<C, E, P>, name: string) => Observable<P> = () =>
  Observable.never()

const commandExecutor = CommandExecutor()
const pondStateTracker = mkNoopPondStateTracker()
describe('FishJar', () => {
  const NUM_EVENTS = 1000
  const NUM_REALTIME_EVENTS = 100

  // fish
  const fishSemantics = Semantics.of('eventStress')
  const eventStress = eventStressFish(fishSemantics)

  // fish for realtimeEvents
  const fishSemantics2 = Semantics.of('eventStress2')
  const eventStress2 = eventStressFish(fishSemantics2)
  const fishName = FishName.of('eventStress')
  const sourceId = SourceId.of('dummy')
  const source = Source.of(eventStress, fishName, sourceId)
  const fishName2 = FishName.of('eventStress2')
  const source2 = Source.of(eventStress2, fishName2, sourceId)

  const sendPassThrough = () => {
    let psn = 0
    return <E>(src: Source, events: ReadonlyArray<E>) =>
      Observable.from(events)
        .map<E, EnvelopeFromStore<E>>(e => ({
          source: src,
          timestamp: Timestamp.of(0),
          lamport: Lamport.of(0),
          id: [fishSemantics, fishName, sourceId],
          psn: Psn.of(psn++),
          payload: e,
        }))
        .toArray()
  }

  const storedEvents = new Array(NUM_EVENTS).fill(0).map<Event>((_, n) => ({
    semantics: source.semantics,
    name: source.name,
    sourceId: source.sourceId,
    timestamp: Timestamp.of(n),
    lamport: Lamport.of(n),
    payload: n,
    // id: [fishSemantics, fishName, sourceId, SequenceNumber.of(n)],
    psn: Psn.of(n),
  }))
  const storedPayloads: ReadonlyArray<number> = storedEvents.map(ev => ev.payload as number)
  const genEvents = () => Observable.of(storedEvents)

  const liveEvents = new Array(NUM_REALTIME_EVENTS).fill(0).map<Event>((_, n) => ({
    // source: source2,
    semantics: source2.semantics,
    sourceId: source2.sourceId,
    name: source2.name,
    timestamp: Timestamp.of(NUM_EVENTS + n),
    lamport: Lamport.of(NUM_EVENTS + n + 1),
    payload: 200 + n,
    // id: [fishSemantics2, fishName2, sourceId, SequenceNumber.of(n)],
    psn: Psn.of(NUM_EVENTS + n + 1), // accomodate previous events and one generated command, however currently psns are not checked
  }))
  const livePayloads: ReadonlyArray<number> = liveEvents.map(ev => ev.payload as number)
  const genRealtimeEvents = () =>
    Observable.concat(Observable.of(liveEvents), Observable.never<ReadonlyArray<Event>>())

  const sourceA: Source = {
    semantics: Semantics.of('a'),
    name: FishName.of(''),
    sourceId: SourceId.of('id'),
  }
  const sub2 = {
    semantics: sourceA.semantics,
    name: FishName.of('foo'),
    sourceId: SourceId.of('s1'),
  }
  const sub4 = {
    semantics: sourceA.semantics,
    name: FishName.of('foo'),
    sourceId: SourceId.of('s2'),
  }
  const multiSourceInitialState: InitialState<State> = () => ({
    state: [],
    subscriptions: [sub2, sub4],
  })

  const multiSourceEventStress = eventStressFishBuilder(fishSemantics, multiSourceInitialState)

  it('should hydrate the single source fishJar and then push some realtime events', async () => {
    const eventStore: EventStore = {
      ...EventStore.noop,
      persistedEvents: genEvents,
      allEvents: genRealtimeEvents,
    }
    const jar = await hydrate(
      FishTypeImpl.downcast(eventStress),
      fishName,
      eventStore,
      SnapshotStore.noop,
      sendPassThrough(),
      fakeObserve,
      commandExecutor,
      pondStateTracker,
    )
      .do(fishJar => fishJar.enqueueCommand([100], noop, noop))
      .toPromise()

    const state = await jar.publicSubject
      .skipWhile(s => s.length < 1101)
      .take(1)
      .toPromise()

    expect(jar.dump().split(' ')[1]).toEqual('1101')
    // the noop-store will assign timestamp zero, so the command will incorrectly be ordered at the front
    expect(state).toEqual([100].concat(storedPayloads).concat(livePayloads))
  })

  it('should hydrate the multi source fishJar and then push some realtime events', async () => {
    const eventStore: EventStore = {
      ...EventStore.noop,
      persistedEvents: genEvents,
      allEvents: genRealtimeEvents,
    }
    const jar = await hydrate(
      FishTypeImpl.downcast(multiSourceEventStress),
      fishName,
      eventStore,
      SnapshotStore.noop,
      sendPassThrough(),
      fakeObserve,
      commandExecutor,
      pondStateTracker,
    )
      .do(fishJar => fishJar.enqueueCommand([100], noop, noop))
      .toPromise()

    const state = await jar.publicSubject
      .skipWhile(s => s.length < 1100)
      .take(1)
      .toPromise()

    expect(jar.dump().split(' ')[1]).toEqual('1100')
    // this fish is not listening to itself, so doesn’t get the command-emitted event
    expect(state).toEqual(storedPayloads.slice().concat(livePayloads))
  })

  it('should properly handle command-event roundtrips', async () => {
    const fish = eventStressFishBuilder(fishSemantics, () => ({
      state: [],
      subscriptions: [Subscription.of(fishSemantics), Subscription.of(fishSemantics2)],
    }))

    const store = EventStore.test(sourceId)
    const sendToStore: SendToStore = (src, events) => {
      const chunk = makeEventChunk(defaultTimeInjector, src, events)
      return store.persistEvents(chunk).map(c => c.map(ev => Event.toEnvelopeFromStore(ev)))
    }

    const jar = await hydrate(
      FishTypeImpl.downcast(fish),
      fishName2,
      store,
      SnapshotStore.noop,
      sendToStore,
      fakeObserve,
      commandExecutor,
      pondStateTracker,
    ).toPromise()

    const stateHas = (f: (s: ReadonlyArray<number>) => boolean) =>
      jar.publicSubject
        .skipWhile(s => !f(s))
        .take(1)
        .toPromise()

    jar.enqueueCommand([1, 2, 3], noop, noop)
    await stateHas(s => s.length >= 3)

    store.directlyPushEvents(liveEvents)
    await stateHas(s => s.length >= 3 + liveEvents.length)

    jar.enqueueCommand([4, 5, 6], noop, noop)
    const state = await stateHas(s => s.length >= 6 + liveEvents.length)

    // the command-events have actual timestamps and should be sorted last
    expect(state).toEqual(livePayloads.slice().concat([1, 2, 3, 4, 5, 6]))
  })
})

describe('SubscriptionLessFishJar', () => {
  const sourceId = SourceId.of('dummy')
  const subscriptionLessFish = FishType.of<number, number, number, number>({
    initialState: () => ({ state: 0, subscriptions: [] }),
    semantics: Semantics.of('ax.subscriptionLess'),
    onCommand: (_, e) => [e],
    onStateChange: OnStateChange.publishPrivateState(),
  })

  const name = FishName.of('subscriptionLess')
  const sendPassThrough = () => {
    let psn = 0
    return <E>(src: Source, events: ReadonlyArray<E>) =>
      Observable.from(events)
        .map<E, EnvelopeFromStore<E>>(e => ({
          source: src,
          timestamp: Timestamp.of(0),
          lamport: Lamport.of(0),
          id: [subscriptionLessFish.semantics, name, sourceId],
          psn: Psn.of(psn++),
          payload: e,
        }))
        .toArray()
  }
  it('should get created when hydrate is called with no subscriptions', async () => {
    const allEventsSpy = jest.fn()
    const persistEventsSpy = jest.fn()
    const persistedEventsSpy = jest.fn()
    const presentSpy = jest.fn()
    const eventStore: EventStore = {
      sourceId,
      allEvents: allEventsSpy,
      persistEvents: persistEventsSpy,
      present: presentSpy,
      persistedEvents: persistedEventsSpy,
    }

    const jar = await hydrate(
      subscriptionLessFish,
      name,
      eventStore,
      SnapshotStore.noop,
      sendPassThrough(),
      fakeObserve,
      commandExecutor,
      pondStateTracker,
    ).toPromise()
    const state = await jar.publicSubject.toPromise()
    expect(state).toEqual(0)
    expect(allEventsSpy).not.toBeCalled()
    expect(presentSpy).not.toBeCalled()
    expect(persistedEventsSpy).not.toBeCalled()
  })
  it('should emit events on command', async () => {
    const allEventsSpy = jest.fn()
    const persistEventsSpy = jest.fn()
    const persistedEventsSpy = jest.fn()
    const presentSpy = jest.fn()
    const eventStore: EventStore = {
      sourceId,
      allEvents: allEventsSpy,
      persistEvents: persistEventsSpy,
      present: presentSpy,
      persistedEvents: persistedEventsSpy,
    }
    const sendSpy: () => Observable<any> = jest.fn(() => Observable.empty())

    const jar = await hydrate(
      subscriptionLessFish,
      name,
      eventStore,
      SnapshotStore.noop,
      sendSpy,
      fakeObserve,
      commandExecutor,
      pondStateTracker,
    ).toPromise()

    jar.enqueueCommand(2, noop, noop)
    expect(allEventsSpy).not.toBeCalled()
    expect(presentSpy).not.toBeCalled()
    expect(persistedEventsSpy).not.toBeCalled()
    expect(persistEventsSpy).not.toBeCalled()
    expect(sendSpy).toBeCalledWith(expect.anything(), [2])
  })
})
