/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  ActyxEvent,
  EventFns,
  EventKey,
  EventsMsg,
  EventsOrTimetravel,
  EventsSortOrder,
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
import * as O from 'fp-ts/Option'
import { clone } from 'ramda'
import { Observable } from '../../node_modules/rxjs'
import * as Rx from '../../node_modules/rxjs'
import log from '../loggers'
import { mkNoopPondStateTracker, PondStateTracker } from '../pond-state'
import { FishErrorReporter, FishId, IsReset } from '../types'
import { cachingReducer } from './cachingReducer'
import { simpleReducer } from './simpleReducer'
import { SnapshotScheduler } from './snapshotScheduler'
import { PendingSnapshot, SerializedStateSnap } from './types'
import { pipe } from 'fp-ts/lib/function'
import { noop } from '../util'

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

/**
 * Observe a Fish using the subscribe_monotonic endpoint (emulated in Actyx <2.12)
 *
 * The rules are:
 * 1. state snapshots are written asynchronously according to the scheduler
 * 1. upon start or after time travel, the latest valid snapshot is used, otherwise initialState, empty offsets, no horizon
 * 1. if isReset is given, query from present to lowerBound to possibly find a newer horizon
 * 1. start subMono from lowerBound with horizon
 * 1. upon time travel, await snapshot persistence and then start over from 2
 */
export const observeMonotonic =
  (
    eventStore: EventFns,
    snapshotStore: SnapshotStore,
    snapshotScheduler: SnapshotScheduler,
    reportFishError: FishErrorReporter,
    pondStateTracker: PondStateTracker = mkNoopPondStateTracker(),
  ) =>
  <S, E>(
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
      attemptStartFrom: FixedStart,
    ): Observable<EventsOrTimetravel<E>> => {
      log.submono.debug(
        'endpoint subscription from',
        fishId,
        attemptStartFrom.horizon ? EventKey.format(attemptStartFrom.horizon) : 'unknown',
      )
      return new Observable<EventsOrTimetravel<E>>((o) =>
        eventStore.subscribeMonotonic(
          { query: subscriptions, sessionId: FishId.canonical(fishId), attemptStartFrom },
          (c) => {
            o.next(c)
            if (c.type == MsgType.timetravel) {
              o.complete()
            }
          },
          (err) => o.error(err),
        ),
      )
    }

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
      await snapshotStore.storeSnapshot(
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
    const monotonicUpdates = (from: FixedStart): Observable<EventsOrTimetravel<E>> => {
      // Run on a scheduler to avoid locking the program up if lots of data are coming in
      const stream = () => endpoint(fishId, where, from).pipe(Rx.subscribeOn(Rx.queueScheduler))

      // Wait for pending snapshot storage requests to finish
      return Rx.from(reducer.awaitPendingPersistence()).pipe(Rx.first(), Rx.concatMap(stream))
    }

    const startingStateMsg = async (timeTravel?: TimeTravelMsg<E>): Promise<StateMsg> => {
      let snap = initialStateSnapshot
      if (timeTravel) {
        await snapshotStore.invalidateSnapshots(fishId.entityType, fishId.name, timeTravel.trigger)
        pipe(
          reducer.latestKnownValidState(timeTravel.trigger),
          // here reducer snapshots are not yet invalidated, that happens when receiving stateMsg
          O.map((localState) => (snap = localState)),
        )
      } // otherwise: initial startup
      if (EventKey.ord.equals(snap.eventKey, EventKey.zero)) {
        // only ask snapshot store if there is no locally cached valid snapshot
        const maybeSnap = await snapshotStore.retrieveSnapshot(
          fishId.entityType,
          fishId.name,
          fishId.version,
        )
        if (maybeSnap) {
          snap = maybeSnap
        }
      }
      if (isReset) {
        let cancel: () => void = noop
        const horizon = await new Promise<EventKey | null>(
          (res) =>
            (cancel = eventStore.queryAllKnownChunked(
              {
                query: where,
                lowerBound: snap.offsets,
                order: EventsSortOrder.Descending,
                horizon: snap.horizon ? EventKey.format(snap.horizon) : undefined,
              },
              1,
              (chunk) => {
                const ev = chunk.events[0]
                if (isResetWrapped(ev)) {
                  res(ev.meta)
                }
              },
              () => res(null),
            )),
        )
        cancel()
        if (
          horizon &&
          (snap.horizon === undefined || EventKey.ord.compare(horizon, snap.horizon) > 0)
        ) {
          log.submono.debug(
            'found horizon',
            EventKey.format(horizon),
            'old:',
            snap.horizon ? EventKey.format(snap.horizon) : 'unknown',
          )
          snap.horizon = horizon
        } else {
          log.submono.debug(
            'kept horizon',
            snap.horizon ? EventKey.format(snap.horizon) : 'unknown',
          )
        }
      }
      return stateMsg(snap)
    }

    const trackingId = FishId.canonical(fishId)

    return makeEndless(monotonicUpdates, startingStateMsg, trackingId).pipe(
      Rx.concatMap((msg) => {
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
          return Rx.throwError(() => err)
        } finally {
          pondStateTracker.eventsFromOtherSourcesProcessingFinished(pondStateTrackerId)
        }
      }),
    )
  }

type StartingStateMsg<E> = (timeTravel?: TimeTravelMsg<E>) => Promise<StateMsg>
type GetMonotonicUpdates<E> = (from: FixedStart) => Observable<EventsOrTimetravel<E>>

const snapshotToFixedStart = (snapshot: LocalSnapshot<unknown>): FixedStart => {
  return {
    from: snapshot.offsets,
    latestEventKey: snapshot.eventKey,
    horizon: snapshot.horizon,
  }
}

// Make a monotonic, limited stream of updates into an unlimited one,
// by chaining the final message (time travel) into a StateMsg and
// automatic restart of the monotonic stream, with appropriate arguments.
const makeEndless = <E>(
  monotonicUpdates: GetMonotonicUpdates<E>,
  startingStateMsg: StartingStateMsg<E>,
  trackingId: string,
): Observable<StateMsg | EventsMsg<E>> =>
  new Observable<StateMsg | EventsMsg<E>>((endlessUpdates) => {
    let latestTimeTravelMsg: TimeTravelMsg<E> | undefined = undefined

    const onError = async (err: any) => {
      log.pond.error('observe stream error', err)

      // On error, just try starting from scratch completely
      // (Should happen very rarely.)
      for (;;) {
        try {
          log.pond.debug('getting startingStateMsg')
          const resetMsg = await startingStateMsg()
          log.pond.debug('got startingStateMsg')
          endlessUpdates.next(resetMsg)

          // This is the error handler, so we know that the old subscription has completed.
          currentSubscription = monotonicUpdates(snapshotToFixedStart(resetMsg.snapshot)).subscribe(
            autoRestartSubscriber,
          )
          break
        } catch (e) {
          log.pond.error('error computing starting state message', e)
        }
      }
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

      complete: async () => {
        if (!latestTimeTravelMsg) {
          return onError(
            new Error(trackingId + ': subscribe_monotonic completed without a time travel message'),
          )
        }

        const msg = latestTimeTravelMsg
        latestTimeTravelMsg = undefined
        log.pond.info(trackingId, 'time traveling due to', EventKey.format(msg.trigger))

        const resetMsg = await startingStateMsg(msg)
        endlessUpdates.next(resetMsg)

        // This is the completion handler, so we know that the old subscription has completed.
        currentSubscription = monotonicUpdates(snapshotToFixedStart(resetMsg.snapshot)).subscribe(
          autoRestartSubscriber,
        )
      },
    }

    // Start the initial subscription
    let currentSubscription = Rx.from(startingStateMsg())
      .pipe(
        Rx.concatMap((start) =>
          Rx.concat(Rx.of(start), monotonicUpdates(snapshotToFixedStart(start.snapshot))),
        ),
      )
      .subscribe(autoRestartSubscriber)

    // Cancel upstream, if this Observable is torn down
    return () => currentSubscription.unsubscribe()
  })
