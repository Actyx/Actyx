/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Actyx, EventKey, Metadata, Offset, OffsetMap, StreamId, TestEvent } from '@actyx/sdk'
import { SnapshotStore } from '@actyx/sdk/lib/snapshotStore'
import { catOptions, chunksOf } from 'fp-ts/lib/Array'
import { none, some } from 'fp-ts/lib/Option'
import { observeMonotonic } from '.'
import { allEvents, Fish, Lamport, Timestamp, Where } from '..'
import { SnapshotScheduler } from './snapshotScheduler'
import { FishErrorReporter } from '../types'
import { interleaveRandom } from '../util'
import { shuffle } from '../util/array'

const numberOfSources = 5
const batchSize = 10
const eventsPerSource = 100
const numberOfIterations = 5
const semanticSnapshotProbability = 0.1
const localSnapshotProbability = 0.05

type SemanticSnapshot = (ev: Payload) => boolean

type Payload = Readonly<{
  stream: string
  sequence: number
  isSemanticSnapshot: boolean
  isLocalSnapshot: boolean
}>

type State = {
  // For asserting that each individual source is delivered in proper order.
  perSource: Record<string, number[]>
  // For asserting that order between sources is correct.
  overall: string[]
}

const onEvent = (state: State, event: Payload, metadata: Metadata) => {
  const { stream, sequence } = event

  const { perSource, overall } = state
  overall.push(metadata.eventId)
  if (perSource[stream] !== undefined) {
    perSource[stream].push(sequence)
  } else {
    perSource[stream] = [sequence]
  }

  return state
}

const timeScale = 1000000
const generateEvents = (count: number) => (stream: StreamId): TestEvent[] =>
  [...new Array(count)].map((_, i) => ({
    offset: Offset.of(i),
    stream,
    tags: [],
    timestamp: Timestamp.of(i * timeScale),
    lamport: Lamport.of(i),
    payload: {
      stream,
      sequence: i,
      isSemanticSnapshot: Math.random() < semanticSnapshotProbability,
      isLocalSnapshot: Math.random() < localSnapshotProbability,
    },
  }))

const mkFish = (isSemanticSnapshot: SemanticSnapshot | undefined): Fish<State, Payload> => ({
  fishId: { entityType: 'some-fish', name: 'some-name', version: 0 },
  where: allEvents as Where<Payload>,
  initialState: { perSource: {}, overall: [] },
  onEvent,
  isReset: isSemanticSnapshot ? (ev, _meta) => isSemanticSnapshot(ev) : undefined,
})

const mkSnapshotScheduler: (
  f: (payload: Payload) => boolean,
  delay?: number,
) => SnapshotScheduler = (f, delay = 0) => ({
  minEventsForSnapshot: 1,
  getSnapshotLevels: (_, ts, limit) =>
    catOptions(
      ts.map((t, i) => {
        const { payload } = (t as unknown) as TestEvent
        if (f(payload as Payload)) {
          return some({ tag: 'x' + i, i, persistAsLocalSnapshot: true })
        } else {
          return none
        }
      }),
    ).filter(x => x.i > limit),
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

const testReportFishError: FishErrorReporter = (err, _fishId, detail) => console.error(err, detail)

type Run = <S>(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  fish: Fish<S, any>,
) => (
  streamId: StreamId,
  events: ReadonlyArray<ReadonlyArray<TestEvent>>,
  snapshotScheduler: SnapshotScheduler,
) => Promise<S>

const hydrate: Run = fish => async (sourceId, events, snapshotScheduler) => {
  const { state: finalState } = await events.reduce(
    async (acc, batch) => {
      const { eventStore, snapshotStore, offsetMap } = await acc
      const offsetMap1 = batch.reduce(OffsetMap.update, { ...offsetMap })

      eventStore.directlyPushEvents(batch)

      const state1 = observeMonotonic(
        eventStore,
        snapshotStore,
        snapshotScheduler,
        testReportFishError,
      )(
        fish.where,
        fish.initialState,
        fish.onEvent,
        fish.fishId,
        fish.isReset,
        fish.deserializeState,
      )
        .first()
        .toPromise()
        .then(swp => swp.state)

      return { state: await state1, offsetMap: offsetMap1, eventStore, snapshotStore }
    },
    Promise.resolve({
      eventStore: Actyx.test({
        nodeId: sourceId,
        // In production, our chunk size is 500.
        // Using small chunks slows down streaming hydration a lot,
        // hence we settle on a rather large number that will still lead to chunking in some cases.
        eventChunkSize: (eventsPerSource * numberOfSources) / 3,
      }),
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
  const eventStore = Actyx.test({ nodeId: sourceId })
  const snapshotStore = SnapshotStore.inMem()

  const observe = observeMonotonic(
    eventStore,
    snapshotStore,
    snapshotScheduler,
    testReportFishError,
  )

  if (intermediates) {
    const states$ = observe(
      fish.where,
      fish.initialState,
      fish.onEvent,
      fish.fishId,
      fish.isReset,
      fish.deserializeState,
    )
      .map(x => x.state)
      .shareReplay(1)

    return events.reduce(async (acc, batch, _i) => {
      await acc

      const res = states$
        .debounceTime(2)
        .take(1)
        .toPromise()
      eventStore.directlyPushEvents(batch)

      return res
    }, Promise.resolve(fish.initialState))
  } else {
    const finalStatePromise = observe(
      fish.where,
      fish.initialState,
      fish.onEvent,
      fish.fishId,
      fish.isReset,
      fish.deserializeState,
    )
      .map(x => x.state)
      .debounceTime(5)
      .first()
      .toPromise()

    events.forEach(eventStore.directlyPushEvents)

    return finalStatePromise
  }
}

const fishConfigs = {
  undefined: mkFish(undefined),
  never: mkFish(() => false),
  random: mkFish((ev: Payload) => ev.isSemanticSnapshot),
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
  const streamIds = [...new Array(numberOfSources)].map(() => StreamId.random())
  const firstStreamId = streamIds[0]
  const events = streamIds.map(generateEvents(eventsPerSource))
  const sorted = [...events.reduce((acc, a) => acc.concat(a), [])].sort(EventKey.ord.compare)

  for (const [name, fish] of Object.entries(fishConfigs)) {
    const expected = hydrate(fish)(firstStreamId, [sorted], neverSnapshotScheduler)

    if (name !== 'random') {
      it(`semantic=${name} expected should be filled`, async () => {
        const e = await expected

        expect(Object.keys(e.perSource).sort()).toEqual([...streamIds].sort())
        expect(e.overall.length).toEqual(eventsPerSource * streamIds.length)
        for (const evs of Object.values(e.perSource)) {
          expect(evs.length).toEqual(eventsPerSource)
        }
      })
    }

    for (const [snapCfgName, scheduler] of Object.entries(snapshotConfigs)) {
      // lockstep local snapshots make only sense with random semantic snapshots
      if (snapCfgName !== 'lockstep' || name === 'random') {
        for (const [runConfigName, run] of Object.entries(runConfigs)) {
          const descr = `semantic=${name}, local=${snapCfgName}, run=${runConfigName}`
          // skip ordered runs that are identical to expected
          if (snapCfgName !== 'no') {
            it(`${descr}, ordered=true`, async () => {
              const result = await run(fish)(firstStreamId, [sorted], scheduler)
              expect(result).toEqual(await expected)
            })
          }
          it(`${descr}, ordered=batch (${numberOfIterations} iterations)`, async () => {
            return Promise.all(
              [...new Array(numberOfIterations)].map(async () => {
                const input = chunksOf(interleaveRandom(events), batchSize)
                const result = await run(fish)(firstStreamId, input, scheduler)
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
                const result = await run(fish)(firstStreamId, input, scheduler)
                expect(result).toEqual(await expected)
              }),
            )
          })
        }
      }
    }
  }
})
