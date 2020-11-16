/* eslint-disable @typescript-eslint/no-explicit-any */
import { clone } from 'ramda'
import { Observable, Scheduler, Subject } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import log from '../loggers'
import { mkNoopPondStateTracker, PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SnapshotScheduler } from '../store/snapshotScheduler'
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
import {
  eventsMonotonic,
  EventsMsg,
  EventsOrTimetravel,
  MsgType,
  StateMsg,
  TimeTravelMsg,
} from './endpoint'
import { PendingSnapshot, SerializedStateSnap, stateWithProvenanceReducer } from './reducer'

// Take some Fish parameters and combine them into a "simpler" onEvent
// with typical reducer signature: (S, E) => S
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

/*
 * Observe a Fish using the subscribe_monotonic endpoint (currently TS impl., but can drop in real impl.)
 *
 * Signature is the same as FishJar.hydrateV2 so we can easily swap it in.
 */
export const observeMonotonic = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  snapshotScheduler: SnapshotScheduler,
  _pondStateTracker: PondStateTracker = mkNoopPondStateTracker(),
) => <S, E>(
  subscriptionSet: SubscriptionSet,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  fishId: FishId,
  isReset?: IsReset<E>,
  deserializeState?: (jsonState: unknown) => S,
): Observable<StateWithProvenance<S>> => {
  const endpoint = eventsMonotonic(eventStore, snapshotStore)

  const { sourceId } = eventStore

  const onEventRaw = mkOnEventRaw(sourceId, clone(initialState), onEvent, isReset)

  const initialStateAsString = JSON.stringify(initialState)
  const initialStateSnapshot = {
    state: initialStateAsString,
    psnMap: OffsetMap.empty,
    eventKey: EventKey.zero,
    horizon: undefined,
    cycle: 0,
  }

  // Here we can find earlier states that we have cached in-process.
  // Returning the initial state is always fine, though. It just leads to more processing.
  const findStartingState = (_before: EventKey): SerializedStateSnap => initialStateSnapshot

  // Create a message that sets the Reducer back to a locally cached state.
  const makeResetMsg = (timeTravel: TimeTravelMsg): StateMsg => {
    const latestValid = findStartingState(timeTravel.trigger)
    return {
      type: MsgType.state,
      snapshot: latestValid,
    }
  }

  const storeSnapshot = async (toStore: PendingSnapshot) => {
    const { snap, tag } = toStore
    snapshotStore.storeSnapshot(
      fishId.entityType,
      fishId.name,
      snap.eventKey,
      snap.psnMap,
      snap.horizon,
      snap.cycle,
      fishId.version,
      tag,
      snap.state,
    )
  }
  // Chain of snapshot storage promises
  let storeSnapshotsPromise: Promise<void> = Promise.resolve()

  const trackingId = FishId.canonical(fishId)

  // The stream of update messages. Should end with a time travel message.
  const monotonicUpdates = (from?: OffsetMap): Observable<EventsOrTimetravel> => {
    const stream = () =>
      endpoint(fishId, subscriptionSet, from)
        // Run on a scheduler to avoid locking the program up if lots of data is coming in.
        .subscribeOn(Scheduler.queue)

    // Wait for pending snapshot storage requests to finish
    return Observable.from(storeSnapshotsPromise)
      .first()
      .concatMap(stream)
  }

  const reducer = stateWithProvenanceReducer(
    onEventRaw,
    initialStateSnapshot,
    snapshotScheduler,
    deserializeState,
  )

  return makeEndless(
    monotonicUpdates,
    makeResetMsg,
    {
      type: MsgType.state,
      snapshot: initialStateSnapshot,
    },
    trackingId,
  ).concatMap(msg => {
    switch (msg.type) {
      case MsgType.state: {
        log.pond.info(
          trackingId,
          'directly setting state.',
          'Num sources:',
          Object.keys(msg.snapshot.psnMap).length,
          '- Cycle:',
          msg.snapshot.cycle,
        )
        reducer.setState(msg.snapshot)
        return []
      }

      case MsgType.events: {
        log.pond.debug(
          trackingId,
          'applying event chunk of size',
          msg.events.length,
          '- caughtUp:',
          msg.caughtUp,
        )
        const s = reducer.appendEvents(msg.events, msg.caughtUp)
        storeSnapshotsPromise = storeSnapshotsPromise.then(async () => {
          await Promise.all(s.snapshots.map(storeSnapshot)).catch(log.pond.warn)
          return
        })
        return s.emit
      }
    }
  })
}

type TimeTravelToStateMsg = (timeTravel: TimeTravelMsg) => StateMsg

type GetMonotonicUpdates = (from?: OffsetMap) => Observable<EventsOrTimetravel>

// Make a monotonic, limited stream of updates into an unlimited one,
// by chaining the final message (time travel) into a StateMsg and
// automatic restart of the monotonic stream, with appropriate arguments.
const makeEndless = (
  monotonicUpdates: GetMonotonicUpdates,
  timeTravelToStateMsg: TimeTravelToStateMsg,
  resetCompletely: StateMsg,
  trackingId: string,
): Observable<StateMsg | EventsMsg> => {
  const endless = new Subject<StateMsg | EventsMsg>()
  const start = new Subject<OffsetMap>()

  start
    .switchMap(x => monotonicUpdates(OffsetMap.isEmpty(x) ? undefined : x))
    .catch(x => {
      log.pond.error(x)
      // On error, just try starting from scratch completely (should happen very rarely.)
      start.next(OffsetMap.empty)
      return Observable.of(resetCompletely)
    })
    .map(x => {
      if (x.type !== MsgType.timetravel) {
        return x
      }
      log.pond.info(trackingId, 'time traveling due to', EventKey.format(x.trigger))
      const resetMsg = timeTravelToStateMsg(x)
      start.next(resetMsg.snapshot.psnMap)
      return resetMsg
    })
    .subscribe(endless)

  start.next(OffsetMap.empty)

  return endless.asObservable().finally(() => {
    start.complete()
    endless.complete()
  })
}
