/* eslint-disable @typescript-eslint/no-explicit-any */
import { clone } from 'ramda'
import { Observable, Scheduler } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import { PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import {
  EventKey,
  FishId,
  IsReset,
  Metadata,
  SourceId,
  StateWithProvenance,
  toMetadata,
} from '../types'
import { eventsMonotonic, EventsOrTimetravel, MsgType } from './endpoint'
import { MonotonicReducer } from './reducer'

const mkOnEventRaw = <S, E>(
  sourceId: SourceId,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  isReset?: IsReset<E>,
) => {
  const metadata = toMetadata(sourceId)

  if (!isReset) {
    return (state: S, ev: Event) => {
      const m = metadata(ev)
      const payload = ev.payload as E

      return onEvent(state, payload, m)
    }
  }

  return (state: S, ev: Event) => {
    const m = metadata(ev)
    const payload = ev.payload as E

    if (isReset(payload, m)) {
      return onEvent(clone(initialState), payload, m)
    } else {
      return onEvent(state, payload, m)
    }
  }
}

export const observeMonotonic = (
  eventStore: EventStore,
  _snapshotStore: SnapshotStore,
  _pondStateTracker: PondStateTracker,
) => <S, E>(
  subscriptionSet: SubscriptionSet,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  _cacheKey: FishId,
  isReset?: IsReset<E>,
  _deserializeState?: (jsonState: unknown) => S,
): Observable<StateWithProvenance<S>> => {
  const endpoint = eventsMonotonic(eventStore)

  const { sourceId } = eventStore

  const onEventRaw = mkOnEventRaw(sourceId, clone(initialState), onEvent, isReset)

  const findInitialState = (_before: EventKey): StateWithProvenance<S> => ({
    state: clone(initialState),
    psnMap: {},
  })

  const reducer = MonotonicReducer(onEventRaw, findInitialState(EventKey.zero))

  const startFromScratch = (from?: OffsetMap): Observable<EventsOrTimetravel> => {
    const s = endpoint(subscriptionSet, from).subscribeOn(Scheduler.queue)
    return Observable.concat(
      s,
      Observable.defer(() => {
        const current = reducer.currentOffsets()
        return startFromScratch(OffsetMap.isEmpty(current) ? undefined : current)
      }),
    )
  }

  // On time travel, switch to a fresh subscribeMonotonic stream
  const updates$ = startFromScratch()

  // .mergeMap(msg => {
  //         if (msg.type === MsgType.timetravel) {
  //             const current = reducer.currentOffsets()
  //             return startFromScratch(OffsetMap.isEmpty(current) ? undefined : current)
  //         } else {
  //             return Observable.of(msg)
  //         }
  //     })

  // const updates$ = Observable.concat(
  //     startFromScratch(),
  //     Observable.defer(() =>

  return updates$.concatMap(msg => {
    switch (msg.type) {
      case MsgType.timetravel: {
        reducer.setState(findInitialState(msg.trigger))
        return []
      }

      case MsgType.state: {
        reducer.setState(msg.state as StateWithProvenance<S>)
        return []
      }

      case MsgType.events: {
        // TODO: Store snapshots (must be async into some pipeline)
        const s = reducer.appendEvents(msg.events)
        if (msg.caughtUp) {
          return [s]
        }
        return []
      }
    }
  })
}
