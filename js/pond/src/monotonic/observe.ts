/* eslint-disable @typescript-eslint/no-explicit-any */
import { none, Option, some } from 'fp-ts/lib/Option'
import { clone } from 'ramda'
import { Observable, Scheduler, Subject } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import log from '../loggers'
import { mkNoopPondStateTracker, PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SnapshotScheduler } from '../store/snapshotScheduler'
import { Where } from '../tagging'
import {
  EventKey,
  FishErrorReporter,
  FishId,
  IsReset,
  LocalSnapshot,
  Metadata,
  StateWithProvenance,
  toMetadata,
} from '../types'
import { cachingReducer } from './cachingReducer'
import {
  eventsMonotonic,
  EventsMsg,
  EventsOrTimetravel,
  FixedStart,
  MsgType,
  StateMsg,
  TimeTravelMsg,
} from './endpoint'
import { simpleReducer } from './simpleReducer'
import { PendingSnapshot, SerializedStateSnap } from './types'

const stateMsg = (latestValid: SerializedStateSnap): StateMsg => ({
  type: MsgType.state,
  snapshot: latestValid,
})

const withErrorHandling = <S, E>(
  fishId: FishId,
  sourceId: string,
  reportFishError: FishErrorReporter,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  isReset?: IsReset<E>,
  deserializeState?: (jsonState: unknown) => S,
) => {
  const mkMetadata = toMetadata(sourceId)
  const castPayload = (ev: Event): E => ev.payload as E

  const onEventWrapped = (state: S, ev: Event) => {
    const metadata = mkMetadata(ev)
    try {
      return onEvent(state, castPayload(ev), metadata)
    } catch (err) {
      reportFishError(err, fishId, { occuredIn: 'onEvent', event: ev, state, metadata })
      throw err
    }
  }

  const isResetWrapped = isReset
    ? (ev: Event) => {
        const metadata = mkMetadata(ev)
        try {
          return isReset(castPayload(ev), metadata)
        } catch (err) {
          reportFishError(err, fishId, { occuredIn: 'isReset', event: ev, metadata })
          throw err
        }
      }
    : () => false

  const deserializeStateWrapped = deserializeState
    ? (jsonState: unknown) => {
        try {
          return deserializeState(jsonState)
        } catch (err) {
          reportFishError(err, fishId, { occuredIn: 'deserializeState', jsonState })
          throw err
        }
      }
    : undefined

  return { onEventWrapped, isResetWrapped, deserializeStateWrapped }
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
  reportFishError: FishErrorReporter,
  pondStateTracker: PondStateTracker = mkNoopPondStateTracker(),
) => <S, E>(
  where: Where<E>,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  fishId: FishId,
  isReset?: IsReset<E>,
  deserializeState?: (jsonState: unknown) => S,
): Observable<StateWithProvenance<S>> => {
  const endpoint = eventsMonotonic(eventStore, snapshotStore)

  const { sourceId } = eventStore

  const initialStateAsString = JSON.stringify(initialState)
  const initialStateSnapshot: SerializedStateSnap = {
    state: initialStateAsString,
    psnMap: OffsetMap.empty,
    eventKey: EventKey.zero,
    horizon: undefined,
    cycle: 0,
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

  const { onEventWrapped, isResetWrapped, deserializeStateWrapped } = withErrorHandling(
    fishId,
    sourceId,
    reportFishError,
    onEvent,
    isReset,
    deserializeState,
  )

  const innerReducer = simpleReducer(
    onEventWrapped,
    {
      state: clone(initialState),
      psnMap: OffsetMap.empty,
      eventKey: EventKey.zero,
      horizon: undefined,
      cycle: 0,
    },
    isResetWrapped,
  )
  const reducer = cachingReducer(
    innerReducer,
    snapshotScheduler,
    storeSnapshot,
    deserializeStateWrapped,
  )

  // The stream of update messages. Should end with a time travel message.
  const monotonicUpdates = (from: Option<FixedStart>): Observable<EventsOrTimetravel> => {
    const stream = () =>
      endpoint(fishId, where, from.toUndefined())
        // Run on a scheduler to avoid locking the program up if lots of data is coming in.
        .subscribeOn(Scheduler.queue)

    // Wait for pending snapshot storage requests to finish
    return Observable.from(reducer.awaitPendingPersistence())
      .first()
      .concatMap(stream)
  }

  const resetToInitialState = stateMsg(initialStateSnapshot)

  const timeTravelToStateMsg = (timeTravel: TimeTravelMsg): StateMsg => {
    const localStartingState = reducer.latestKnownValidState(timeTravel.trigger, timeTravel.high)

    return localStartingState.fold(resetToInitialState, stateMsg)
  }

  const trackingId = FishId.canonical(fishId)

  return makeEndless(
    monotonicUpdates,
    timeTravelToStateMsg,
    resetToInitialState,
    trackingId,
  ).concatMap(msg => {
    const pondStateTrackerId = pondStateTracker.eventsFromOtherSourcesProcessingStarted(
      fishId.entityType,
      fishId.name,
    )
    try {
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

          const s = reducer.appendEvents(msg.events)
          return msg.caughtUp ? [s] : []
        }
      }
    } catch (err) {
      return Observable.throw(err)
    } finally {
      pondStateTracker.eventsFromOtherSourcesProcessingFinished(pondStateTrackerId)
    }
  })
}

type TimeTravelToStateMsg = (timeTravel: TimeTravelMsg) => StateMsg

type GetMonotonicUpdates = (from: Option<FixedStart>) => Observable<EventsOrTimetravel>

const snapshotToFixedStart = (snapshot: LocalSnapshot<unknown>): Option<FixedStart> => {
  if (OffsetMap.isEmpty(snapshot.psnMap)) {
    return none
  }

  return some({
    from: snapshot.psnMap,
    latestEventKey: snapshot.eventKey,
    horizon: snapshot.horizon,
  })
}

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
  const start = new Subject<Option<FixedStart>>()

  start
    .switchMap(monotonicUpdates)
    .catch(x => {
      log.pond.error(x)
      // On error, just try starting from scratch completely (should happen very rarely.)
      start.next(undefined)
      return Observable.of(resetCompletely)
    })
    .map(x => {
      if (x.type !== MsgType.timetravel) {
        return x
      }
      log.pond.info(trackingId, 'time traveling due to', EventKey.format(x.trigger))
      const resetMsg = timeTravelToStateMsg(x)
      start.next(snapshotToFixedStart(resetMsg.snapshot))
      return resetMsg
    })
    .subscribe(endless)

  start.next(none)

  return endless.asObservable().finally(() => {
    start.complete()
    endless.complete()
  })
}
