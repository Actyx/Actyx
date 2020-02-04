import { Observable } from 'rxjs'
import { noop } from '../util'
import { PondStateTracker } from './pond-state'

export const mkNoopPondStateTracker = (): PondStateTracker => ({
  hydrationStarted: () => '',
  hydrationFinished: noop,
  commandProcessingStarted: () => '',
  commandProcessingFinished: noop,
  eventsFromOtherSourcesProcessingStarted: () => '',
  eventsFromOtherSourcesProcessingFinished: noop,
  observe: () => Observable.never(),
})
