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
  LocalSnapshot,
  Metadata,
  SourceId,
  StateWithProvenance,
  toMetadata,
} from '../types'
import { eventsMonotonic, EventsMsg, MsgType, StateMsg } from './endpoint'
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

  // Here we can find earlier states that we have cached in-process.
  const findStartingState = (_before: EventKey): LocalSnapshot<S> => ({
    state: clone(initialState),
    psnMap: {},
    cycle: 0,
    eventKey: EventKey.zero,
    horizon: undefined,
  })

  const reducer = MonotonicReducer(onEventRaw, findStartingState(EventKey.zero))

  // On time travel, switch to a fresh subscribeMonotonic stream
  const updates = (from?: OffsetMap): Observable<StateMsg | EventsMsg> =>
    endpoint(subscriptionSet, from)
      .subscribeOn(Scheduler.queue)
      .concatMap(msg => {
        if (msg.type === MsgType.timetravel) {
          const latestValid = findStartingState(msg.trigger)
          const resetMsg: StateMsg = {
            type: MsgType.state,
            state: latestValid,
          }

          const startFrom = latestValid.psnMap

          return Observable.concat(
            Observable.of(resetMsg),
            updates(OffsetMap.isEmpty(startFrom) ? undefined : startFrom),
          )
        } else {
          return [msg]
        }
      })
      .catch(err => {
        console.log(err) // Improve me

        // Terminate normally and let the code further below take care of restarting
        return Observable.empty()
      })

  // If the stream of updates terminates without a timetravel message – due to an error or the ws engine –,
  // then we can just restart it.
  const updates$ = Observable.concat(updates(), Observable.defer(updates))

  return updates$.concatMap(msg => {
    switch (msg.type) {
      case MsgType.state: {
        reducer.setState(msg.state as StateWithProvenance<S>)
        return []
      }

      case MsgType.events: {
        // TODO: Store snapshots
        const s = reducer.appendEvents(msg.events)
        if (msg.caughtUp) {
          return [s]
        }
        return []
      }
    }
  })
}
