/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import * as R from 'ramda'
import { Observable } from 'rxjs'
import * as seedrandom from 'seedrandom'
import { Subscription } from './'
import { Event as EventStoreEvent, Events } from './eventstore/types'
import log from './loggers'
import { Pond } from './pond'
import {
  Envelope,
  FishName,
  FishType,
  InitialState,
  Lamport,
  OnEvent,
  OnStateChange,
  Psn,
  Semantics,
  SourceId,
  SourceIdTag,
  Timestamp,
} from './types'
import { Opaque } from './util/opaqueTag'

//#region test fish definition
export type State = number

export type Command = never

export type Event = number

const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  log.pond.info('got event', event, ' and state ', state)
  return state + event.payload
}

const testFishInitialState: (subs: ReadonlyArray<Subscription>) => InitialState<State> = (
  subs: ReadonlyArray<Subscription>,
) => (_name: string, _sourceId: Opaque<string, typeof SourceIdTag>) => ({
  state: 0,
  subscriptions: subs,
})

const createTestFish = (
  semantics: Semantics,
  subs: ReadonlyArray<Subscription>,
): FishType<Command, Event, State> =>
  FishType.of<State, Command, Event, State>({
    semantics,
    initialState: testFishInitialState(subs),
    onEvent,
    onStateChange: OnStateChange.publishPrivateState(),
    localSnapshot: undefined,
    semanticSnapshot: undefined,
  })

// this fish subscribes to events from semantics 'a', name 'foo', source 's1'
const simpleFish = createTestFish(Semantics.of('none'), [
  {
    semantics: Semantics.of('a'),
    name: FishName.of('foo'),
    sourceId: SourceId.of('s1'),
  },
])

// this fish subscribes to events from semantics 'a', on source 's1' but no definite name (wildcard)
const simpleFishNoName = createTestFish(Semantics.of('none'), [
  Subscription.of(Semantics.of('a'), undefined, SourceId.of('s1')),
])

// this fish subscribes to events from semantics 'a', name 'foo', no source (wildcard)
const simpleFishNoSource = createTestFish(Semantics.of('none'), [
  Subscription.of(Semantics.of('a'), FishName.of('foo')),
])

const allSources: ReadonlyArray<
  Readonly<{ semantics: Semantics; name: FishName; sourceId: SourceId }>
> = [
  {
    semantics: Semantics.of('a'),
    name: FishName.of('foo'),
    sourceId: SourceId.of('s1'),
  },
  {
    semantics: Semantics.of('a'),
    name: FishName.of('bar'),
    sourceId: SourceId.of('s1'),
  },
  {
    semantics: Semantics.of('b'),
    name: FishName.of('foo'),
    sourceId: SourceId.of('s1'),
  },
  {
    semantics: Semantics.of('b'),
    name: FishName.of('bar'),
    sourceId: SourceId.of('s1'),
  },
  {
    semantics: Semantics.of('a'),
    name: FishName.of('foo'),
    sourceId: SourceId.of('s2'),
  },
  {
    semantics: Semantics.of('a'),
    name: FishName.of('bar'),
    sourceId: SourceId.of('s2'),
  },
  {
    semantics: Semantics.of('b'),
    name: FishName.of('foo'),
    sourceId: SourceId.of('s2'),
  },
  {
    semantics: Semantics.of('b'),
    name: FishName.of('bar'),
    sourceId: SourceId.of('s2'),
  },
]

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
    payload: event,
    timestamp,
    lamport: Lamport.of(timestamp),
  }
}

const createEvents = (
  sources: ReadonlyArray<Readonly<{ semantics: Semantics; name: FishName; sourceId: SourceId }>>,
  events: number,
): Events => {
  const state = GeneratorState.create()
  return Array(events)
    .fill(undefined)
    .map((_value, index) =>
      createEventStoreEvent(
        state,
        sources[index % sources.length].semantics,
        sources[index % sources.length].name,
        sources[index % sources.length].sourceId,
        1,
        Timestamp.of(index * 3600 * 24 * 1_000_000),
      ),
    )
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
  const events = R.flatten(createEvents(allSources, nEvents))
  const fishName = 'ignore'
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
    await Observable.timer(50, 10)
      .take(1)
      .toPromise()
    const state1 = await pond
      .observe(type, fishName)
      .take(1)
      .toArray()
      .toPromise()
    states.push(state1[0])

    await pond.dispose()
  }
  return states
}

const sameStateCheck = async <C, E, P>(
  seed: string,
  type: FishType<C, E, P>,
  nEvents: number,
  nShuffles: number,
  stateEqualTo: string,
): Promise<void> => {
  const states = await snapshotCheck(seed, type, nEvents, nShuffles)
  const state = states[0]
  expect(state).not.toEqual(0)
  const expected = new Array(nShuffles).fill(state)
  expect(states).toEqual(expected)
  expect(JSON.stringify(states[0])).toEqual(stateEqualTo)
}

describe('applying events in random order', () => {
  const nEvents = 1000
  const nShuffles = 2
  it(
    'should ultimately produce the same state',
    async () => {
      await sameStateCheck('seed', simpleFish, nEvents, nShuffles, '125') // precise subscription, gets only every eight event, so 1000/8 = 125
      await sameStateCheck('seed', simpleFishNoName, nEvents, nShuffles, '250') // name wildcard, two names, so every fourth event, 1000/4 = 250 in total
      await sameStateCheck('seed', simpleFishNoSource, nEvents, nShuffles, '250') // source wildcard, two sources, every fourth event, 1000/4 = 250 in total
    },
    60000,
  )
})
