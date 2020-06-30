/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as R from 'ramda'
import { Observable } from 'rxjs'
import * as seedrandom from 'seedrandom'
import { Event as EventStoreEvent, Events } from './eventstore/types'
import log from './loggers'
import { Pond } from './pond'
import { Subscription } from './subscription'
import {
  Envelope,
  FishName,
  FishType,
  Lamport,
  OnEvent,
  OnStateChange,
  Psn,
  Semantics,
  SemanticSnapshot,
  SnapshotFormat,
  SourceId,
  Timestamp,
} from './types'

//#region test fish definition
export type State = number

export type Command = never

export type Event =
  | {
      type: 'set'
      value: number
    }
  | {
      type: 'add'
      value: number
    }

const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  log.pond.info('got event', event)
  switch (event.payload.type) {
    case 'set':
      return event.payload.value
    case 'add':
      return state + event.payload.value
  }
}

const isSemanticSnapshot = () => (ev: Envelope<Event>): boolean => ev.payload.type === 'set'
const snapshotFormat = SnapshotFormat.identity<State>(0)

const createTestFish = (
  semantics: Semantics,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  localSnapshot?: SnapshotFormat<State, any>,
  semanticSnapshot?: SemanticSnapshot<Event>,
): FishType<Command, Event, State> =>
  FishType.of<State, Command, Event, State>({
    semantics,
    initialState: () => ({ state: 0, subscriptions: [Subscription.of(semantics)] }),
    onEvent,
    onStateChange: OnStateChange.publishPrivateState(),
    localSnapshot,
    semanticSnapshot,
  })

const noSnapshots = createTestFish(Semantics.of('none'))
const localSnapshots = createTestFish(Semantics.of('local'), snapshotFormat)
const semanticSnapshots = createTestFish(Semantics.of('local'), undefined, isSemanticSnapshot)
const bothSnapshots = createTestFish(Semantics.of('local'), snapshotFormat, isSemanticSnapshot)
//#endregion
//#region fake pubsub generation
type GeneratorState = {
  psns: { [source: string]: number }
  fsns: { [source: string]: { [semantics: string]: { [name: string]: number } } }
}
const GeneratorState = {
  create: (): GeneratorState => ({ psns: {}, fsns: {} }),
}
const createEventStoreEvent = (
  generatorState: GeneratorState,
  semantics: Semantics,
  fishName: FishName,
  source: SourceId,
  event: Event,
  timestamp: Timestamp,
): EventStoreEvent => {
  const { psns, fsns } = generatorState
  const psn = psns[source] || 0

  psns[source] = psn + 1
  const sequence: number = R.path([source, semantics, fishName], generatorState.fsns) || 0

  generatorState.fsns = R.assocPath([source, semantics, fishName], sequence + 1, fsns)
  return {
    sourceId: source,
    psn: Psn.of(psn),
    semantics,
    name: fishName,
    tags: [],
    payload: event,
    timestamp,
    lamport: Lamport.of(timestamp),
  }
}

const createEvents = (
  r: seedrandom.prng,
  semantics: Semantics,
  fishName: FishName,
  sources: SourceId[],
  events: number,
): Events[] => {
  const state = GeneratorState.create()
  const result: Events[] = sources.map(source =>
    Array(events)
      .fill(undefined)
      .map((_value, index) =>
        createEventStoreEvent(
          state,
          semantics,
          fishName,
          source,
          r.double() < 0.05 ? { type: 'set', value: r.int32() } : { type: 'add', value: r.int32() },
          Timestamp.of(index * 3600 * 24 * 1_000_000),
        ),
      ),
  )
  return result
}
//#endregion
//#region util
const shuffle = <T>(a: ReadonlyArray<T>, r: seedrandom.prng): ReadonlyArray<T> => {
  const aa = [...a]
  let x: T
  for (let i = aa.length - 1; i > 0; i--) {
    const j = Math.floor(r.double() * (i + 1))

    x = aa[i]

    aa[i] = aa[j]

    aa[j] = x
  }
  return aa
}

//#endregion

const snapshotCheck = async <C, E, P>(
  seed: string,
  type: FishType<C, E, P>,
  nEvents: number,
  nShuffles: number,
): Promise<P[]> => {
  const rand = seedrandom(seed)
  const fishName = FishName.of('a')
  const fishName2 = FishName.of('b')
  const sources = [SourceId.of('s1'), SourceId.of('s2'), SourceId.of('s3')]
  const events = R.flatten(createEvents(rand, type.semantics, fishName, sources, nEvents))
  const states: P[] = []
  for (let i = 0; i < nShuffles; i += 1) {
    // todo: shuffle will not usually lead to interesting time travel behaviour
    const shuffled = shuffle(events, rand)
    const pond = await Pond.test()
    // wake up the fish named fishName
    await pond
      .observe(type, fishName)
      .take(1)
      .toPromise()
    // send the shuffled events
    pond.directlyPushEvents(shuffled)
    // wait some time until the events are pushed through
    await Observable.timer(10, 10)
      .take(1)
      .toPromise()
    const state1 = await pond
      .observe(type, fishName)
      .take(1)
      .toPromise()

    // wake up a fish with the same events once we got all events
    const state2 = await pond
      .observe(type, fishName2)
      .take(1)
      .toPromise()

    states.push(state1)
    states.push(state2)

    await pond.dispose()
  }
  return states
}

const sameStateCheck = async <C, E, P>(
  seed: string,
  type: FishType<C, E, P>,
  nEvents: number,
  nShuffles: number,
): Promise<void> => {
  const states = await snapshotCheck(seed, type, nEvents, nShuffles)
  const state = states[0]
  expect(state).not.toEqual(0)
  const expected = new Array(nShuffles * 2).fill(state)
  expect(states).toEqual(expected)
}

describe('applying events in random order', () => {
  const nEvents = 10
  const nShuffles = 10
  it(
    'should ultimately produce the same state',
    async () => {
      await sameStateCheck('seed', noSnapshots, nEvents, nShuffles)
      await sameStateCheck('seed', localSnapshots, nEvents, nShuffles)
      await sameStateCheck('seed', semanticSnapshots, nEvents, nShuffles)
      await sameStateCheck('seed', bothSnapshots, nEvents, nShuffles)
    },
    100000,
  )
})
