/* eslint-disable @typescript-eslint/no-explicit-any */
import { Observable, Subject } from 'rxjs'
import { CommandApi, UnsafeAsync } from './commandApi'
import { EventStore } from './eventstore'
import { Event } from './eventstore/types'
import { CommandExecutor } from './executors/commandExecutor'
import { hydrate } from './fishJar'
import log from './loggers'
import { defaultTimeInjector, makeEventChunk, SendToStore } from './pond'
import {
  mkNoopPondStateTracker,
  mkPondStateTracker,
  PondState,
  PondStateTracker,
} from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { EnvelopeFromStore } from './store/util'
import { Subscription } from './subscription'
import { eventStressFish, eventStressFishBuilder, State } from './testkit/eventStressFish'
import {
  Envelope,
  FishName,
  FishType,
  FishTypeImpl,
  InitialState,
  Lamport,
  OnCommand,
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

  const sendPassThroughSwallow = () => {
    // tslint:disable-next-line no-let
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

  const sendPassThrough = (mockInternalStore: Subject<EnvelopeFromStore<any>[]>) => {
    // tslint:disable-next-line no-let
    let psn = 0
    return <E>(src: Source, events: ReadonlyArray<E>) =>
      Observable.from(events)
        .map<E, EnvelopeFromStore<E>>(e => ({
          source: src,
          timestamp: Timestamp.of(0),
          lamport: Lamport.of(0),
          id: [fishSemantics, fishName, sourceId],
          semantics: fishSemantics,
          name: fishName,
          sourceId: src.sourceId,
          psn: Psn.of(psn++),
          payload: e,
        }))
        .toArray()
        .do(resultArray => mockInternalStore.next(resultArray))
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
  const genRealtimeEvents = () => Observable.of(liveEvents)

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
    const mockInternalStore = new Subject<EnvelopeFromStore<any>[]>()

    const eventStore: EventStore = {
      ...EventStore.noop,
      persistedEvents: genEvents,
      allEvents: () =>
        Observable.concat(
          genRealtimeEvents(),
          mockInternalStore.map(evs => evs.map(Event.fromEnvelopeFromStore)),
        ),
    }
    const jar = await hydrate(
      FishTypeImpl.downcast(eventStress),
      fishName,
      eventStore,
      SnapshotStore.noop,
      sendPassThrough(mockInternalStore),
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
      sendPassThroughSwallow(),
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

  describe('error handling', () => {
    const mkBorked = (
      onCommand: OnCommand<number, number, number> = (_: number, c: number) => [c],
      onEvent = (s: number, e: Envelope<number>) => s + e.payload,
    ) =>
      FishType.of<number, number, number, number>({
        initialState: () => ({ state: 0 }),
        semantics: Semantics.of('ax.borked'),
        onCommand,
        onEvent,
        onStateChange: OnStateChange.publishPrivateState(),
      })

    const getJar = async (fish: FishType<number, number, number>, tracker: PondStateTracker) => {
      const store = EventStore.test(sourceId)
      const sendToStore: SendToStore = (src, events) => {
        const chunk = makeEventChunk(defaultTimeInjector, src, events)
        return store.persistEvents(chunk).map(c => c.map(ev => Event.toEnvelopeFromStore(ev)))
      }

      return hydrate(
        FishTypeImpl.downcast(fish),
        fishName2,
        store,
        SnapshotStore.noop,
        sendToStore,
        fakeObserve,
        commandExecutor,
        tracker,
      ).toPromise()
    }

    const assertUnblocked = async (tracker: PondStateTracker) => {
      const pondState = await tracker
        .observe()
        .take(1)
        .toPromise()

      expect(PondState.isBusy(pondState)).toBeFalsy()
    }

    it('should update the state tracker even when command application has an error', async () => {
      const onCommand = (_: number, c: number) => {
        if (c === 5) {
          throw new Error('that was my internal wrong number')
        }

        return [c]
      }

      const borkedFish = mkBorked(onCommand)
      const tracker = mkPondStateTracker(log.pond)

      const jar = await getJar(borkedFish, tracker)

      jar.enqueueCommand(4, noop, noop)

      try {
        jar.enqueueCommand(5, noop, noop)
      } catch (_) {
        // We get an immediate error.
      }

      // ...but UI is not blocked,
      await assertUnblocked(tracker)

      // and we can also still interact with the jar.
      const s = jar.publicSubject.take(2).toPromise()
      jar.enqueueCommand(6, noop, noop)
      return expect(s).resolves.toEqual(10)
    })

    it('should update the state tracker even when command application has an async error', async () => {
      const onCommand = (_: number, c: number) => {
        if (c === 5) {
          // return CommandApi.http.get('this is not a url').chain(() => CommandApi.of([c]))
          return UnsafeAsync(
            Observable.timer(10).map(() => {
              throw new Error('hello')
            }),
          ).chain(() => CommandApi.of([c]))
        }

        return [c]
      }

      const borkedFish = mkBorked(onCommand)
      const tracker = mkPondStateTracker(log.pond)

      const jar = await getJar(borkedFish, tracker)

      jar.enqueueCommand(4, noop, noop)
      jar.enqueueCommand(5, noop, noop)
      jar.enqueueCommand(6, noop, noop)

      // Async errors kindly leave the jar alive
      const q = await jar.publicSubject
        .filter(s => s === 10)
        .take(1)
        .toPromise()
      expect(q).toEqual(10)

      // ...and also do not block the UI indefinitely.
      await assertUnblocked(tracker)
    })

    it('should update the state tracker even when event application has an error', async () => {
      const onEvent = (s: number, e: Envelope<number>) => {
        if (e.payload === 5) {
          throw new Error('this is my internal wrong number')
        }

        return s + e.payload
      }

      const borkedFish = mkBorked(undefined, onEvent)
      const tracker = mkPondStateTracker(log.pond)

      const jar = await getJar(borkedFish, tracker)

      jar.enqueueCommand(4, noop, noop)
      jar.enqueueCommand(5, noop, noop)
      jar.enqueueCommand(6, noop, noop)

      try {
        await jar.publicSubject.skipWhile(s => s < 10).toPromise()
      } catch (_) {
        // Jar is broken now if we try to read up to the final state which should be 10.
      }

      // But we can still retrieve the previous state...
      const nextState = await jar.publicSubject.take(1).toPromise()
      expect(nextState).toEqual(4)

      // ...and also do not block the UI indefinitely.
      await assertUnblocked(tracker)
    })
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
    // tslint:disable-next-line no-let
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
      ...EventStore.noop,
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
      ...EventStore.noop,
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
