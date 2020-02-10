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
    Pond.mock().then(async pond => {
      const r = op(pond)
      if (r instanceof Promise) {
        return r.then(async q => {
          await pond.dispose()
          return q
        })
      } else {
        await pond.dispose()
        return r
      }
    })
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
    await Promise.all([commands, events, awaitPendingDBOperations]).then(([c, e, db]) => {
      expect(c).toEqual(expectedCommands)
      expect(e).toEqual(expectedEvents)
      expect(db).toEqual('done')
    })
    return pond.dispose()
  })

  it('should allow listening to state changes of other fishes', () =>
    withPond(async pond => {
      const setup = pond
        .observe(timerFishListenerFishType, 'dory')
        .take(2)
        .toArray()
        .toPromise()

      // switch on the listener
      await pond
        .feed(timerFishListenerFishType, FishName.of('dory'))({ type: 'enable' })
        .toPromise()

      expect(await setup).toEqual([{ type: 'disabled' }, { count: 0, type: 'enabled' }])

      // observe state changes of the listener
      const result = pond
        .observe(timerFishListenerFishType, 'dory')
        .take(4)
        .toArray()
        .toPromise()

      // switch on the timer fish
      // must wait for the command to complete before we start observing the listener,
      // due to race condition reasons.
      await pond
        .feed(timerFishType, FishName.of('nemo'))({ type: 'enable' })
        .toPromise()

      const states = await result
      expect(states).toEqual([
        { count: 0, type: 'enabled' },
        { count: 1, type: 'enabled' },
        { count: 2, type: 'enabled' },
        { count: 3, type: 'enabled' },
      ])
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
      await pond.dispose()
      // eslint-disable-next-line @typescript-eslint/ban-ts-ignore
      // @ts-ignore implementation detail, how to test it without exposing private details?
      expect(Object.keys(pond.jars).length).toEqual(0)
    }))
})
