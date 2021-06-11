/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  ActyxEvent,
  EventFns,
  EventKey,
  EventsMsg,
  EventsOrTimetravel,
  FixedStart,
  LocalSnapshot,
  Metadata,
  MsgType,
  OffsetMap,
  StateMsg,
  StateWithProvenance,
  TimeTravelMsg,
  Where,
} from '@actyx/sdk'
import { SnapshotStore } from '@actyx/sdk/lib/snapshotStore'
import { none, Option, some } from 'fp-ts/lib/Option'
import { clone } from 'ramda'
import { Observable, Scheduler } from 'rxjs'
import log from '../loggers'
import { mkNoopPondStateTracker, PondStateTracker } from '../pond-state'
import { FishErrorReporter, FishId, IsReset } from '../types'
import { cachingReducer } from './cachingReducer'
import { simpleReducer } from './simpleReducer'
import { SnapshotScheduler } from './snapshotScheduler'
import { PendingSnapshot, SerializedStateSnap } from './types'

const stateMsg = (latestValid: SerializedStateSnap): StateMsg => ({
  type: MsgType.state,
  snapshot: latestValid,
})

const withErrorHandling = <S, E>(
  fishId: FishId,
  reportFishError: FishErrorReporter,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  isReset?: IsReset<E>,
  deserializeState?: (jsonState: unknown) => S,
) => {
  const castPayload = (ev: ActyxEvent): E => ev.payload as E

  const onEventWrapped = (state: S, ev: ActyxEvent) => {
    try {
      return onEvent(state, castPayload(ev), ev.meta)
    } catch (err) {
      reportFishError(err, fishId, { occuredIn: 'onEvent', event: ev, state, metadata: ev.meta })
      throw err
    }
  }

  const isResetWrapped = isReset
    ? (ev: ActyxEvent) => {
        try {
          return isReset(castPayload(ev), ev.meta)
        } catch (err) {
          reportFishError(err, fishId, { occuredIn: 'isReset', event: ev, metadata: ev.meta })
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
 */
export const observeMonotonic = (
  eventStore: EventFns,
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
  const endpoint = <E>(
    fishId: FishId,
    subscriptions: Where<E>,
    attemptStartFrom?: FixedStart,
  ): Observable<EventsOrTimetravel<E>> =>
    new Observable<EventsOrTimetravel<E>>(o =>
      eventStore.subscribeMonotonic(
        { query: subscriptions, sessionId: FishId.canonical(fishId), attemptStartFrom },
        c => {
          o.next(c)
          if (c.type == MsgType.timetravel) {
            o.complete()
          }
        },
      ),
    )

  const initialStateAsString = JSON.stringify(initialState)
  const initialStateSnapshot: SerializedStateSnap = {
    state: initialStateAsString,
    offsets: OffsetMap.empty,
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
      snap.offsets,
      snap.horizon,
      snap.cycle,
      fishId.version,
      tag,
      snap.state,
    )
  }

  const { onEventWrapped, isResetWrapped, deserializeStateWrapped } = withErrorHandling(
    fishId,
    reportFishError,
    onEvent,
    isReset,
    deserializeState,
  )

  const innerReducer = simpleReducer(
    onEventWrapped,
    {
      state: clone(initialState),
      offsets: OffsetMap.empty,
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
  const monotonicUpdates = (from: Option<FixedStart>): Observable<EventsOrTimetravel<E>> => {
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

  const timeTravelToStateMsg = (timeTravel: TimeTravelMsg<E>): StateMsg => {
    const localStartingState = reducer.latestKnownValidState(
      timeTravel.trigger.meta,
      timeTravel.high.meta,
    )

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
            'Num streams:',
            Object.keys(msg.snapshot.offsets).length,
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

type TimeTravelToStateMsg<E> = (timeTravel: TimeTravelMsg<E>) => StateMsg

type GetMonotonicUpdates<E> = (from: Option<FixedStart>) => Observable<EventsOrTimetravel<E>>

const snapshotToFixedStart = (snapshot: LocalSnapshot<unknown>): Option<FixedStart> => {
  if (OffsetMap.isEmpty(snapshot.offsets)) {
    return none
  }

  return some({
    from: snapshot.offsets,
    latestEventKey: snapshot.eventKey,
    horizon: snapshot.horizon,
  })
}

// Make a monotonic, limited stream of updates into an unlimited one,
// by chaining the final message (time travel) into a StateMsg and
// automatic restart of the monotonic stream, with appropriate arguments.
const makeEndless = <E>(
  monotonicUpdates: GetMonotonicUpdates<E>,
  timeTravelToStateMsg: TimeTravelToStateMsg<E>,
  resetCompletely: StateMsg,
  trackingId: string,
): Observable<StateMsg | EventsMsg<E>> =>
  new Observable<StateMsg | EventsMsg<E>>(endlessUpdates => {
    let latestTimeTravelMsg: TimeTravelMsg<E> | undefined = undefined

    const onError = (err: any) => {
      log.pond.error(err)

      // On error, just try starting from scratch completely
      // (Should happen very rarely.)
      endlessUpdates.next(resetCompletely)

      // This is the error handler, so we know that the old subscription has completed.
      currentSubscription = monotonicUpdates(none).subscribe(autoRestartSubscriber)
    }

    const autoRestartSubscriber = {
      next: (msg: EventsOrTimetravel<E>) => {
        if (msg.type === MsgType.timetravel) {
          latestTimeTravelMsg = msg
          // Now we expect stream to complete...
          return
        }

        endlessUpdates.next(msg)
      },

      error: onError,

      complete: () => {
        if (!latestTimeTravelMsg) {
          return onError(
            new Error(trackingId + ': subscribe_monotonic completed without a time travel message'),
          )
        }

        const msg = latestTimeTravelMsg

        log.pond.info(trackingId, 'time traveling due to', EventKey.format(msg.trigger.meta))

        // Reset for next time
        latestTimeTravelMsg = undefined

        const resetMsg = timeTravelToStateMsg(msg)

        endlessUpdates.next(resetMsg)

        // This is the completion handler, so we know that the old subscription has completed.
        currentSubscription = monotonicUpdates(snapshotToFixedStart(resetMsg.snapshot)).subscribe(
          autoRestartSubscriber,
        )
      },
    }
    // Start the initial subscription
    let currentSubscription = monotonicUpdates(none).subscribe(autoRestartSubscriber)

    // Cancel upstream, if this Observable is torn down
    return () => currentSubscription.unsubscribe()
  })
