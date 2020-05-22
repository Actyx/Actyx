/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { catOptions, chunksOf } from 'fp-ts/lib/Array'
import { none, some } from 'fp-ts/lib/Option'
import { FishName, Psn, Semantics, SourceId, SubscriptionSet, Timestamp } from './'
import { Event, Events, EventStore, OffsetMap } from './eventstore'
import { includeEvent } from './eventstore/testEventStore'
import { interleaveRandom, intoOrderedChunks } from './eventstore/utils'
import { FishEventStore, FishInfo } from './fishEventStore'
import { SnapshotStore } from './snapshotStore'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { EnvelopeFromStore } from './store/util'
import { EventKey, Lamport, OnEvent, Envelope } from './types'
import { shuffle } from './util/array'
import produce, { enableMapSet } from 'immer'

enableMapSet()

const numberOfSources = 5
const batchSize = 10
const eventsPerSource = 200
const numberOfIterations = 20
const semanticSnapshotProbability = 0.1
const localSnapshotProbability = 0.05

type SemanticSnapshot<E> = (env: EnvelopeFromStore<E>) => boolean

type Payload = Readonly<{
  sequence: number
  isSemanticSnapshot: boolean
  isLocalSnapshot: boolean
}>

type State = Map<string, number[]>
const initialState: State = new Map()
const onEvent: OnEvent<State, Payload> = (state, envelope) =>
  produce(state, draft => myOnEvent(draft, envelope))

const myOnEvent = (state: State, envelope: Envelope<Payload>): void => {
  const {
    source: { sourceId },
    payload: { sequence },
  } = envelope

  const seqs = state.get(sourceId) || []
  if (seqs.length === 0) {
    state.set(sourceId, seqs)
  }
  seqs.push(sequence)
}

const timeScale = 1000000
const generateEvents = (count: number) => (sourceId: SourceId): Events =>
  [...new Array(count)].map((_, i) => ({
    psn: Psn.of(i),
    semantics: Semantics.of('foo'),
    sourceId,
    name: FishName.of('foo'),
    timestamp: Timestamp.of(i * timeScale),
    lamport: Lamport.of(i),
    payload: {
      sequence: i,
      isSemanticSnapshot: Math.random() < semanticSnapshotProbability,
      isLocalSnapshot: Math.random() < localSnapshotProbability,
    },
  }))

const mkFish = (
  isSemanticSnapshot: SemanticSnapshot<Payload> | undefined,
): FishInfo<State, Payload> => ({
  semantics: Semantics.of('some-fish'),
  fishName: FishName.of('some-name'),
  subscriptionSet: SubscriptionSet.all,
  initialState,
  onEvent,
  isSemanticSnapshot,
  snapshotFormat: {
    version: 1,
    serialize: x => Array.from(x),
    deserialize: (serialized: [string, number[]][]) => new Map(serialized),
  },
})

const mkSnapshotScheduler: (
  f: (payload: Payload) => boolean,
  delay?: number,
) => SnapshotScheduler = (f, delay = 0) => ({
  minEventsForSnapshot: 1,
  getSnapshotLevels: (_, ts, limit) =>
    catOptions(
      ts.map((t, i) => {
        const { payload } = (t as unknown) as EnvelopeFromStore<Payload>
        if (f(payload)) {
          return some({ tag: 'x' + i, i, persistAsLocalSnapshot: true })
        } else {
          return none
        }
      }),
    )
      .filter(x => x.i > limit)
      .reverse(),
  // Delay = 0 means we always store directly
  isEligibleForStorage: (snap, latest) => {
    const delta = latest.timestamp - snap.timestamp
    return delta >= delay * timeScale
  },
})

const neverSnapshotScheduler: SnapshotScheduler = {
  minEventsForSnapshot: 1, // Still have this scheduler be called.
  getSnapshotLevels: (_, _ts) => [],
  isEligibleForStorage: () => {
    throw new Error('Should not be called!')
  },
}

type Run = <S>(
  fish: FishInfo<S, Payload>,
) => (
  sourceId: SourceId,
  events: ReadonlyArray<Events>,
  snapshotScheduler: SnapshotScheduler,
) => Promise<S>

const hydrate: Run = fish => async (sourceId, events, snapshotScheduler) => {
  const { state: finalState } = await events.reduce(
    async (acc, batch) => {
      const { eventStore, snapshotStore, offsetMap } = await acc
      const offsetMap1 = batch.reduce(includeEvent, { ...offsetMap })

      eventStore.directlyPushEvents(batch)
      const store = await FishEventStore.initialize(
        fish,
        eventStore,
        snapshotStore,
        snapshotScheduler,
        offsetMap1,
      ).toPromise()
      const state1 = await store
        .currentState()
        .toPromise()
        .then(sp => sp.state)
      return { state: state1, offsetMap: offsetMap1, eventStore, snapshotStore }
    },
    Promise.resolve({
      eventStore: EventStore.test(
        sourceId,
        // In production, our chunk size is 500.
        // Using small chunks slows down streaming hydration a lot,
        // hence we settle on a rather large number that will still lead to chunking in some cases.
        (eventsPerSource * numberOfSources) / 3,
      ),
      snapshotStore: SnapshotStore.inMem(),
      offsetMap: OffsetMap.empty,
      state: fish.initialState,
    }),
  )

  return finalState
}

const live: (intermediateStates: boolean) => Run = intermediates => fish => async (
  sourceId,
  events,
  snapshotScheduler,
) => {
  const eventStore = EventStore.test(sourceId) // todo inmem?
  const snapshotStore = SnapshotStore.inMem()
  const store = await FishEventStore.initialize(
    fish,
    eventStore,
    snapshotStore,
    snapshotScheduler,
    OffsetMap.empty,
  ).toPromise()

  return events.reduce(async (acc, batch, i) => {
    await acc

    eventStore.directlyPushEvents(batch) // Make available for rehydration.

    // FES expects every batch to be sorted internally.
    // To assure this we use the same procedure as the fishJar does.
    const sortedChunks = intoOrderedChunks(batch)

    let n = false

    for (const sortedEvents of sortedChunks) {
      const sortedEnvelopes: EnvelopeFromStore<Payload>[] = sortedEvents.map(e =>
        Event.toEnvelopeFromStore(e),
      )

      n = store.processEvents(sortedEnvelopes) || n
    }

    const isLast = i === events.length - 1
    return (n && intermediates) || isLast
      ? store
          .currentState()
          .toPromise()
          .then(sp => sp.state)
      : acc
  }, Promise.resolve(fish.initialState))
}

const fishConfigs = {
  undefined: mkFish(undefined),
  never: mkFish(() => false),
  random: mkFish(({ payload: { isSemanticSnapshot } }) => isSemanticSnapshot),
}
const runConfigs = { hydrate, live: live(false), liveIntermediateStates: live(true) }
const snapshotConfigs = {
  no: neverSnapshotScheduler,
  random: mkSnapshotScheduler(({ isLocalSnapshot }) => isLocalSnapshot),
  randomDelayed: mkSnapshotScheduler(
    ({ isLocalSnapshot }) => isLocalSnapshot,
    1 + Math.random() * 15,
  ),
  lockstep: mkSnapshotScheduler(({ isSemanticSnapshot }) => isSemanticSnapshot),
}

describe(`the fish event store with randomized inter-source event ordering`, () => {
  const sourceIds = [...new Array(numberOfSources)].map(() => SourceId.random())
  const sourceId = sourceIds[0]
  const events = sourceIds.map(generateEvents(eventsPerSource))
  const sorted = [...events.reduce((acc, a) => acc.concat(a), [])].sort(EventKey.ord.compare)

  for (const [name, fish] of Object.entries(fishConfigs)) {
    const expected = hydrate(fish)(sourceId, [sorted], neverSnapshotScheduler)

    for (const [snapCfgName, scheduler] of Object.entries(snapshotConfigs)) {
      // lockstep local snapshots make only sense with random semantic snapshots
      if (snapCfgName !== 'lockstep' || name === 'random') {
        for (const [runConfigName, run] of Object.entries(runConfigs)) {
          const descr = `semantic=${name}, local=${snapCfgName}, run=${runConfigName}`
          // skip ordered runs that are identical to expected
          if (snapCfgName !== 'no') {
            it(`${descr}, ordered=true`, async () => {
              const result = await run(fish)(sourceId, [sorted], scheduler)
              expect(result).toEqual(await expected)
            })
          }
          it(`${descr}, ordered=batch (${numberOfIterations} iterations)`, async () => {
            return Promise.all(
              [...new Array(numberOfIterations)].map(async () => {
                const input = chunksOf(interleaveRandom(events), batchSize)
                const result = await run(fish)(sourceId, input, scheduler)
                expect(result).toEqual(await expected)
              }),
            )
          })
          it(`${descr}, ordered=false (${numberOfIterations} iterations)`, async () => {
            return Promise.all(
              [...new Array(numberOfIterations)].map(async () => {
                // We assume that at least a single source will not timetravel;
                // otherwise local snapshot functionality breaks.
                const input = chunksOf(interleaveRandom(events), batchSize).map(shuffle)
                const result = await run(fish)(sourceId, input, scheduler)
                expect(result).toEqual(await expected)
              }),
            )
          })
        }
      }
    }
  }
})
