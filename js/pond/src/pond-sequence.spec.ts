import { Observable } from 'rxjs'
import { Pond } from './pond'
import {
  Envelope,
  FishName,
  FishType,
  InitialState,
  LegacyStateChange,
  OnCommand,
  OnEvent,
  OnStateChange,
  Semantics,
  StateEffect,
  Timestamp,
} from './types'

export type State = { n: number }

export type Command = { type: 'set'; n: number } | { type: 'check'; n: number }

export type Event = { type: 'set'; n: number }

const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  switch (event.payload.type) {
    case 'set':
      if (state.n !== event.payload.n - 1) {
        throw new Error(`expected state to be ${event.payload.n - 1}, but was ${state.n}`)
      }
      return { n: event.payload.n }
  }
}

const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'set':
      if (state.n !== command.n - 1) {
        throw new Error(`expected state to be ${command.n - 1}, but was ${state.n}`)
      }
      return [{ type: 'set', n: command.n }]
    case 'check':
      if (state.n !== command.n) {
        throw new Error(`expected state to be ${command.n}, but was ${state.n}`)
      }
      return []
  }
}

const checkTimer = (n: number): Observable<StateEffect<Command, State>> => {
  return Observable.timer(0, 10).map<number, StateEffect<Command, State>>(() =>
    StateEffect.sendSelf<Command>({ type: 'check', n }),
  )
}

const onStateChange: LegacyStateChange<State, Command, State> = state => {
  const name = `pipeline${state.n}`
  return [{ name, create: () => checkTimer(state.n) }]
}

const initialState: InitialState<State> = () => ({ state: { n: 0 } })

export const sequenceTestFish: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>({
  semantics: Semantics.of('sequenceTestFish'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: OnStateChange.legacy(onStateChange),
})

async function testSlow(): Promise<void> {
  const N = 30
  const pond = await Pond.mock()
  let cumulativeFeedTime = 0
  let cumulativeFeedTimeLast10 = 0
  let prevTime = Timestamp.now()
  for (let i = 0; i < N; i++) {
    await pond
      .feed(sequenceTestFish, FishName.of('test'))({ type: 'set', n: i + 1 })
      .take(1)
      .toPromise()
    const currTime0 = Timestamp.now()
    cumulativeFeedTime += currTime0 - prevTime
    if (i > N - 11) cumulativeFeedTimeLast10 += currTime0 - prevTime
    prevTime = currTime0
    await Observable.timer(30).toPromise()
    const currTime = Timestamp.now()
    if (currTime - prevTime > 100000)
      console.info('very long delay delta: ', (currTime - prevTime) / 1000, ' ms')
    prevTime = currTime
  }
  console.info(
    'Average time of fish feeding: ',
    cumulativeFeedTime / N / 1000,
    ' ms; last 10 executions: ',
    cumulativeFeedTimeLast10 / 10000,
    ' ms',
  )
  await pond.dispose()
}

describe('application of commands in the pond', () => {
  // FIXME
  it.skip('should be processing events interleaved with commands (fast)', async () => {
    const N = 100
    const pond = await Pond.mock()
    for (let i = 0; i < N; i++) {
      pond
        .feed(sequenceTestFish, FishName.of('test'))({ type: 'set', n: i + 1 })
        .subscribe()
    }
    return pond

      ._events()
      .do(console.info)
      .take(N)
      .toArray()
      .toPromise()
      .then(events => {
        for (let i = 1; i < events.length; i++) {
          if (events[i - 1].payload.n !== events[i].payload.n - 1) {
            throw new Error()
          }
        }
      })
  })
  // FIXME
  it.skip(
    'should be processing events interleaved with commands (fast2)',
    async () => {
      const N = 1000
      const pond = await Pond.mock()
      for (let i = 0; i < N; i++) {
        pond
          .feed(sequenceTestFish, FishName.of('test'))({ type: 'set', n: i + 1 })
          .subscribe()
        pond
          .feed(sequenceTestFish, FishName.of('test'))({ type: 'check', n: i + 1 })
          .subscribe()
      }
      return pond

        ._events()
        .take(N)
        .toArray()
        .toPromise()
        .then(events => {
          for (let i = 1; i < events.length; i++) {
            if (events[i - 1].payload.n !== events[i].payload.n - 1) {
              throw new Error()
            }
          }
        })
    },
    10000,
  )

  it('should be processing events interleaved with commands (delayed)', () => testSlow(), 20000)
})
