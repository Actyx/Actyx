/* eslint-disable @typescript-eslint/no-explicit-any */
import { clone } from 'ramda'
import { Observable, ReplaySubject, Subject } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import { PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import { FishId, IsReset, Metadata, SourceId, StateWithProvenance, toMetadata } from '../types'
import { eventsMonotonic, MsgType } from './endpoint'
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
  isReset?: IsReset<E>, // IMPLEMENT ME
  _deserializeState?: (jsonState: unknown) => S,
): Observable<StateWithProvenance<S>> => {
  const endpoint = eventsMonotonic(eventStore)

  const { sourceId } = eventStore

  const onEventRaw = mkOnEventRaw(sourceId, clone(initialState), onEvent, isReset)

  const reducer = MonotonicReducer(onEventRaw, initialState)

  const out: Subject<StateWithProvenance<S>> = new ReplaySubject(1)

  let startingPoint: OffsetMap | undefined = undefined
  const startFromScratch = (): Observable<unknown> => {
    return endpoint(subscriptionSet, startingPoint).do(msg => {
      switch (msg.type) {
        case MsgType.state:
          reducer.setState(msg.state as StateWithProvenance<S>)
          return

        case MsgType.events: {
          const states = reducer.appendEvents(msg.events)
          out.next(states.latest)
          return
        }

        case MsgType.timetravel:
          startingPoint = reducer.timeTravel(msg.trigger)
          return
      }
    })
  }

  // This could be phrased in terms of a mergeScan, but there’d be no gain in clarity imho.
  startFromScratch()
    .finally(startFromScratch)
    .subscribe()

  return out
}
