import { Observable } from 'rxjs'
import { Pond } from './pond'
import {
  Envelope,
  FishName,
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  PondObservables,
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

const checkStateAfter10ms = (pond: PondObservables<State>) =>
  pond.observeSelf().concatMap(state =>
    // First state will be immediately updated, so we must not check that one.
    (state.n > 0 ? Observable.timer(10) : Observable.empty()).mapTo(
      StateEffect.sendSelf<Command>({ type: 'check', n: state.n }),
    ),
  )

const initialState: InitialState<State> = () => ({ state: { n: 0 } })

const fishConfig = {
  semantics: Semantics.of('sequenceTestFish'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: checkStateAfter10ms,
}
export const sequenceTestFish: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>(fishConfig)

async function testSlow(): Promise<void> {
  const N = 30
  const pond = await Pond.test()
  let cumulativeFeedTime = 0
  let cumulativeFeedTimeLast10 = 0
  let prevTime = Timestamp.now()
  // We need to observe the fish in the pond for the onStateChange pipeline to do anything.
  pond.observe(sequenceTestFish, FishName.of('test')).subscribe()
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
  const expectNEvents = (pond: Pond, N: number) =>
    pond
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
      .then(() => pond.dispose())

  it('should be processing events interleaved with commands (fast)', async () => {
    const N = 100
    const pond = await Pond.test()

    // _events is not a ReplaySubject, so we must start listening before everything else.
    const res = expectNEvents(pond, 50)

    // We need to observe the fish in the pond for the onStateChange pipeline to do anything.
    pond.observe(sequenceTestFish, FishName.of('test')).subscribe()
    for (let i = 0; i < N; i++) {
      pond
        .feed(sequenceTestFish, FishName.of('test'))({ type: 'set', n: i + 1 })
        .subscribe()
      await Observable.timer(20).toPromise()
    }

    return res
  })

  it(
    'should be processing events interleaved with commands (fast2)',
    async () => {
      const N = 1000
      const pond = await Pond.test()

      // _events is not a ReplaySubject, so we must start listening before everything else.
      const res = expectNEvents(pond, 100)

      const dofeed = pond.feed(sequenceTestFish, FishName.of('test'))
      // Since we are not subscribed to the fish, its onStateChange is not triggered.
      // Otherwise, it would throw Errors, since we have no delay between the set-commands.
      for (let i = 0; i < N; i++) {
        dofeed({ type: 'set', n: i + 1 }).subscribe()
        dofeed({ type: 'check', n: i + 1 }).subscribe()
      }
      return res
    },
    10000,
  )

  it('should be processing events interleaved with commands (delayed)', () => testSlow(), 20000)
})
