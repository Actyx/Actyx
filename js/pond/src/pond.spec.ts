import { Observable, ReplaySubject } from 'rxjs'
import { CommandApi } from '.'
import {
  ArticleConfig,
  articleFishType,
  Command as ArticleCommand,
  ConfigReply,
} from './articleFish.support.test'
import log from './loggers'
import { Pond, TimeInjector } from './pond'
import { brokenTimerFishType, timerFishType } from './timerFish.support.test'
import { timerFishListenerFishType } from './timerFishListener.support.test'
import {
  FishName,
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  Semantics,
  Source,
  Target,
  Timestamp,
} from './types'
import { unreachableOrElse } from './util/'

type TestCommand = { type: 'config' } | ConfigReply
type TestEvent = { type: 'gotArticleConfig'; config: ArticleConfig }
type TestState = { readonly self: Target<ConfigReply> }

const initialState: InitialState<TestState> = name => ({
  state: {
    self: Target.of(testFishType, FishName.of(name)),
  },
  subscriptions: [],
})

const onEvent: OnEvent<TestState, TestEvent> = (state, event) => {
  const { payload } = event
  switch (payload.type) {
    case 'gotArticleConfig':
      return state
    default:
      return unreachableOrElse(payload.type, state)
  }
}

const onCommand: OnCommand<TestState, TestCommand, TestEvent> = (state, command) => {
  switch (command.type) {
    case 'config': {
      return CommandApi.pond.send<ArticleCommand>(Target.of(articleFishType, state.self.name))({
        type: 'getConfig',
        replyTo: state.self,
      })
    }
    case 'configReply':
      return [{ type: 'gotArticleConfig', config: command.config }]
    default:
      return unreachableOrElse(command, [])
  }
}

const testFishType: FishType<TestCommand, TestEvent, TestState> = FishType.of({
  semantics: Semantics.of('test'),
  initialState,
  onEvent,
  onCommand,
})
// IMPORTANT: The zero injector might be dangerous because the sorting of all events
// will be weird due to no difference in timestamp.
// So don't use this except for tests here.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const zeroTimeInjector: TimeInjector = (_source: Source, _events: ReadonlyArray<any>) =>
  Timestamp.of(0)

async function delayFor(ms: number): Promise<void> {
  await _sleep(ms)
}
const _sleep = (ms: number) => {
  return new Promise(resolve => setTimeout(resolve, ms))
}

describe(`Pond ('the usual')`, () => {
  const withPond = <T>(op: (pond: Pond) => T | Promise<T>): Promise<T> =>
    Pond.mock().then(pond => op(pond))
  it('should run fish interactions', () =>
    withPond(pond => {
      const sourceId = pond.info().sourceId
      const expectedCommands = [
        // From what I see, the order doesn't matter
        { command: 'configReply', fish: 'test/E9120356' },
        { command: 'getConfig', fish: 'article/E9120356' },
      ]
      const expectedEvents = [{ event: 'gotArticleConfig', fish: `test/E9120356/${sourceId}` }]
      const commands = pond
        .commands()
        .map(sc => ({
          fish: `${sc.target.semantics.semantics}/${sc.target.name}`,
          command: sc.command.type,
        }))
        .take(2)
        .toArray()
        .toPromise()
      const events = pond
        // tslint:disable-next-line:deprecation
        ._events()
        .map(ev => ({ fish: Source.format(ev.source), event: ev.payload.type }))
        .take(1)
        .toArray()
        .toPromise()
      pond
        .feed(testFishType, FishName.of('E9120356'))({ type: 'config' })
        .subscribe({ error: err => log.pond.error(err) })
      return Promise.all([commands, events]).then(([c, e]) => {
        expect(c).toEqual(expectedCommands)
        expect(e).toEqual(expectedEvents)
      })
    }))

  it('should run fish interactions with fake time', async () => {
    const pond = await Pond.mock({ timeInjector: zeroTimeInjector })
    const sourceId = pond.info().sourceId
    const expectedCommands = [
      { command: 'configReply', fish: 'test/E9120356' },
      { command: 'getConfig', fish: 'article/E9120356' },
    ]
    const expectedEvents = [{ event: 'gotArticleConfig', fish: `test/E9120356/${sourceId}` }]
    const commands = pond
      .commands()
      .map(sc => ({
        fish: `${sc.target.semantics.semantics}/${sc.target.name}`,
        command: sc.command.type,
      }))
      .take(2)
      .toArray()
      .toPromise()
    const events = pond
      // tslint:disable-next-line:deprecation
      ._events()
      .map(ev => ({ fish: Source.format(ev.source), event: ev.payload.type }))
      .take(1)
      .toArray()
      .toPromise()
    pond
      .feed(testFishType, FishName.of('E9120356'))({ type: 'config' })
      .subscribe({ error: err => log.pond.error(err) })

    // we need to wait for the db query above to complete
    const awaitPendingDBOperations = delayFor(200).then(() => {
      log.pond.info('slept')
      return 'done'
    })
    return Promise.all([commands, events, awaitPendingDBOperations]).then(([c, e, db]) => {
      expect(c).toEqual(expectedCommands)
      expect(e).toEqual(expectedEvents)
      expect(db).toEqual('done')
    })
  })

  it('should call onStateChange', () =>
    withPond(pond => {
      // we start listening to events when this function is invoked, but can consume later
      const events = () => {
        // using ReplaySubject as a buffer
        const s = new ReplaySubject()
        const sub = pond
          // tslint:disable-next-line:deprecation
          ._events()
          .map(ev => ev.payload.type as string)
          .subscribe(s)
        // this will clean up the ReplaySubject once this stream is canceled
        return s.finally(() => sub.unsubscribe())
      }
      return Observable.of(
        events()
          .take(5)
          .toArray(),
      )
        .concatMap(evs =>
          pond
            .feed(timerFishType, FishName.of('bruce'))({ type: 'enable' })
            .mapTo(evs),
        )
        .concatMap(evs => evs)
        .concatMap(evs => {
          expect(evs).toEqual(['enabled', 'pinged', 'pinged', 'pinged', 'pinged'])
          const allGood = Observable.of('all good now').delay(500)
          const nextEvs = events()
            .merge(allGood)
            .take(3)
            .toArray()
          return pond
            .feed(timerFishType, FishName.of('bruce'))({ type: 'disable' })
            .mapTo(nextEvs)
        })
        .concatMap(evs => evs)
        .filter(x => x instanceof Array)
        .do(evs => {
          expect(evs).toEqual(['disabled', 'reset', 'enabled'])
        })
        .toPromise()
    }))

  it('should allow listening to state changes of other fishes', () =>
    withPond(async pond => {
      // observe state changes of the listener
      const result = pond.observe(timerFishListenerFishType, 'dory')
      // switch on the listener
      pond
        .feed(timerFishListenerFishType, FishName.of('dory'))({ type: 'enable' })
        .subscribe({ error: err => log.pond.error(err) })
      // switch on the timer fish
      pond
        .feed(timerFishType, FishName.of('nemo'))({ type: 'enable' })
        .subscribe({ error: err => log.pond.error(err) })
      const states = await result
        .take(3)
        .toArray()
        .toPromise()
      expect(states[1]).toEqual({ count: 2, type: 'enabled' })
    }))

  it('should use semantics of first fish in onCommand', () =>
    withPond(pond =>
      pond
        .feed(timerFishType, FishName.of('nemo'))({ type: 'enable' })
        .toPromise()
        .then(() =>
          pond
            .feed(brokenTimerFishType, FishName.of('nemo'))({ type: 'enable' })
            .toPromise(),
        ),
    ))

  it('should have a dump function', () =>
    withPond(pond =>
      pond
        .feed(timerFishType, FishName.of('nemo'))({ type: 'enable' })
        .toPromise()
        .then(() =>
          pond
            .dump()
            .toArray()
            .toPromise()
            .then(lines => expect(lines.length).toEqual(1)),
        ),
    ))

  it('should have an info function', () =>
    withPond(pond => {
      expect(pond.info().sourceId).toEqual('MOCK')
      return Promise.resolve()
    }))

  it('should dispose event store subscription', () =>
    withPond(async pond => {
      await pond.dispose().toPromise()
      // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
      // @ts-ignore implementation detail, how to test it without exposing private details?
      expect(Object.keys(pond.jars).length).toEqual(0)
    }))
})
