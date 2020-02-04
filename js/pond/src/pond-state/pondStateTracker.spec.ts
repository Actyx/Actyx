import { FishName, Milliseconds, Semantics } from '../types'
import { Loggers } from '../util'
import { PondState, PondStateTracker } from './pond-state'
import { mkInitialState, mkPondStateTracker } from './pondStateTracker'

const fishName = FishName.of('fishy')
const semantics = Semantics.of('test')

const mkTestTime = () => {
  let time = 0

  return () => {
    time += 1
    return Milliseconds.of(time)
  }
}

const mk = () => {
  const log = Loggers.testLoggers()
  const tracker = mkPondStateTracker(log, mkTestTime())
  return { log, tracker }
}

const s = (x: PondStateTracker): Promise<PondState> =>
  x
    .observe()
    .take(1)
    .toPromise()

const e = (x: PondStateTracker, result: object): Promise<void> =>
  expect(s(x)).resolves.toMatchObject(result)

describe('mkPondStateTracker', () => {
  it('emits initial state', () => expect(s(mk().tracker)).resolves.toEqual(mkInitialState()))

  it('tracks when fish starts hydration', () => {
    const { tracker } = mk()
    const key = tracker.hydrationStarted(semantics, fishName)
    return e(tracker, {
      hydration: {
        numBeingProcessed: 1,
        fish: {
          [key]: true,
        },
      },
    })
  })

  it('tracks when fish stops hydrating', () => {
    const { tracker } = mk()
    const key = tracker.hydrationStarted(semantics, fishName)
    tracker.hydrationFinished(key)
    return e(tracker, {
      hydration: {
        numBeingProcessed: 0,
        fish: {},
      },
    })
  })

  it('should warn when trying to stop hydration for an unknown fish', () => {
    const { tracker, log } = mk()
    tracker.hydrationFinished('unknown')
    expect(log.warnings).toEqual(['"Hydration ended for an unknown fish with key: %s.":"unknown"'])
  })

  it('tracks when pond starts processing command', () => {
    const { tracker } = mk()
    const key = tracker.commandProcessingStarted(semantics, fishName)
    return e(tracker, {
      commands: {
        numBeingProcessed: 1,
        fish: {
          [key]: true,
        },
      },
    })
  })

  it('tracks when pond finishes processing command', () => {
    const { tracker } = mk()
    const key = tracker.commandProcessingStarted(semantics, fishName)
    tracker.commandProcessingFinished(key)
    return e(tracker, {
      commands: {
        numBeingProcessed: 0,
        fish: {},
      },
    })
  })

  it('should warn when trying to finish processing an unknown command', () => {
    const { tracker, log } = mk()
    tracker.commandProcessingFinished('unknown')
    expect(log.warnings).toEqual([
      '"Command processing finished for an unknown command with key: %s.":"unknown"',
    ])
  })

  it('tracks when pond starts processing event chunks', () => {
    const { tracker } = mk()
    const key = tracker.eventsFromOtherSourcesProcessingStarted(semantics, fishName)
    return e(tracker, {
      eventsFromOtherSources: {
        numBeingProcessed: 1,
        fish: {
          [key]: true,
        },
      },
    })
  })

  it('tracks when pond finishes processing event chunks', () => {
    const { tracker } = mk()
    const key = tracker.eventsFromOtherSourcesProcessingStarted(semantics, fishName)
    tracker.eventsFromOtherSourcesProcessingFinished(key)
    return e(tracker, {
      eventsFromOtherSources: {
        numBeingProcessed: 0,
        fish: {},
      },
    })
  })

  it('should warn when trying to finish processing an unknown event chunk', () => {
    const { tracker, log } = mk()
    tracker.eventsFromOtherSourcesProcessingFinished('unknown')
    expect(log.warnings).toEqual([
      '"Events from other sources processing finished for an unknown chunk with key: %s.":"unknown"',
    ])
  })

  it('should track state', () => {
    const { tracker } = mk()
    const key1 = tracker.hydrationStarted(semantics, fishName)
    tracker.hydrationFinished(key1)
    tracker.commandProcessingStarted(semantics, fishName)
    const key2 = tracker.commandProcessingStarted(semantics, fishName)
    tracker.eventsFromOtherSourcesProcessingStarted(semantics, fishName)
    tracker.hydrationStarted(semantics, FishName.of('another one'))

    tracker.commandProcessingFinished(key2)

    return expect(s(tracker)).resolves.toEqual({
      commands: { fish: { 'test:fishy:2': true }, numBeingProcessed: 1 },
      eventsFromOtherSources: { fish: { 'test:fishy:4': true }, numBeingProcessed: 1 },
      hydration: { fish: { 'test:another one:5': true }, numBeingProcessed: 1 },
    })
  })
})
