/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import * as R from 'ramda'
import { clone } from 'ramda'
import { Observable, Subject, Subscription as RxSubscription } from 'rxjs'
import { catchError, tap } from 'rxjs/operators'
import { EventStore } from './eventstore'
import { AllEventsSortOrders, Event, Events } from './eventstore/types'
import { intoOrderedChunks } from './eventstore/utils'
import { FishEventStore, FishInfo } from './fishEventStore'
import log from './loggers'
import { PondStateTracker } from './pond-state'
import { SnapshotStore } from './snapshotStore'
import { SnapshotScheduler } from './store/snapshotScheduler'
import { Subscription, SubscriptionSet } from './subscription'
import {
  FishId,
  FishName,
  IsReset,
  Metadata,
  Psn,
  Semantics,
  SnapshotFormat,
  Source,
  SourceId,
  StateWithProvenance,
  Timestamp,
} from './types'
import { lookup } from './util'
import { runStats } from './util/runStats'

export const mkSubscriptionSet = (source: Source, subscriptions?: ReadonlyArray<Subscription>) => {
  // filter out subscriptions to jelly fish, which are not equal to source
  // jelly events are considered ephemeral, that's why we prohibit subscribing to them
  const subscriptions0 =
    subscriptions &&
    subscriptions.filter(s0 => !Semantics.isJelly(s0.semantics) || R.equals(source, s0))
  // add a self-subscription in case the list is empty
  const subscriptions1 = subscriptions0 && subscriptions0.length > 0 ? subscriptions0 : [source]
  return SubscriptionSet.or(subscriptions1)
}

// I is an intermediate value that is consumed by the specialized command handling logic.
// Pond V1 has Async vs. SyncCommandResult, while V2 has Payload+Tags.
export type CommandFn<S, I> = (state: S) => I

export type FishJar<C, E, P> = Readonly<{
  // enqueue the commands for processing
  enqueueCommand: (command: C, onComplete: () => void, onError: (err: any) => void) => void

  // public "state"
  publicSubject: Observable<P>

  dispose: () => void

  dump: () => string
}>

type CommandInput<S, I> = Readonly<{
  type: 'command'
  command: CommandFn<S, I>
  onComplete: () => void
  onError: (err: any) => void
}>

type EventScanState<S, E> = Readonly<{
  eventStore: FishEventStore<S, E>
  /**
   * Sometimes we do not have to emit a new state. E.g. when a command
   * does not result in events.
   */
  emit: ReadonlyArray<StateWithProvenance<S>>
}>

const mkEventScanAcc = <S, E>(
  pondStateTracker: PondStateTracker,
  semantics: Semantics,
  name: FishName,
) => {
  // Aggregate incoming events into ever-new states.
  // We reveal the Provenance too, so that downstream consumers can implement specialized logic.
  const evScanAcc = (
    current: EventScanState<S, E>,
    events: Events,
  ): Observable<EventScanState<S, E>> => {
    const start = Timestamp.now()
    const pondStateTrackerEventProcessingToken = pondStateTracker.eventsFromOtherSourcesProcessingStarted(
      semantics,
      name,
    )

    const unblock = () =>
      pondStateTracker.eventsFromOtherSourcesProcessingFinished(
        pondStateTrackerEventProcessingToken,
      )

    try {
      const profile = `inject-events/${semantics}`

      runStats.durations.start(profile, start)
      const needsState = current.eventStore.processEvents(events)
      runStats.durations.end(profile, start, Timestamp.now())

      const result = needsState
        ? current.eventStore
            .currentState()
            .pipe(runStats.profile.profileObservable(`inject-compute/${semantics}`))
            .map(s => ({
              ...current,
              emit: [s],
            }))
        : Observable.of({ ...current, emit: [] })

      return result.pipe(
        tap(
          unblock,
          // On errors, also update the tracker
          unblock,
        ),
      )
    } catch (e) {
      // Synchronous error, for example from onEvent
      unblock()
      throw e
    }
  }

  return evScanAcc
}

export type CommandPipeline<S, I> = Readonly<{
  // Subject where new commands must be pushed
  subject: Subject<CommandInput<S, I>>

  // Subscription to the running pipeline (cancel to destroy pipeline)
  subscription: RxSubscription
}>

type CommandScanState = Readonly<{
  waitFor: Psn
}>

const commandPipeline = <S, I>(
  pondStateTracker: PondStateTracker,
  localSourceId: SourceId,
  semantics: string,
  name: string,
  handler: ((input: I) => Observable<Events>),
  stateSubject: Observable<StateWithProvenance<S>>,
  eventFilter: ((t: Event) => boolean),
): CommandPipeline<S, I> => {
  const commandIn: Subject<CommandInput<S, I>> = new Subject()

  // Command handling pipeline. After each command, if it emitted any events that we are subscribed to,
  // the handling of the following command is delayed until upstream (event aggregation) has seen and
  // integrated the event into our state.
  // In this way, we arrive at our core command guarantee: Every command sees all local effects of all
  // preceding commands.
  const cmdScanAcc = (
    current: CommandScanState,
    input: CommandInput<S, I>,
  ): Observable<CommandScanState> => {
    const { command, onComplete, onError } = input

    const pondStateTrackerCommandProcessingToken = pondStateTracker.commandProcessingStarted(
      semantics,
      name,
    )
    const unblock = () => {
      pondStateTracker.commandProcessingFinished(pondStateTrackerCommandProcessingToken)
    }

    const result = stateSubject
      .filter(stateWithProvenance => {
        if (current.waitFor < Psn.zero) {
          return true
        }

        const latestSeen = lookup(stateWithProvenance.psnMap, localSourceId)
        const pass = latestSeen !== undefined && latestSeen >= current.waitFor

        if (!pass) {
          log.pond.debug(
            semantics,
            '/',
            name,
            '/',
            localSourceId,
            'waiting for',
            current.waitFor,
            '; currently at:',
            latestSeen,
          )
        }
        return pass
      })
      .map(sp => sp.state)
      .take(1)
      .concatMap(s => {
        const onCommandResult = command(s)
        const stored = handler(onCommandResult)

        return stored.concatMap(envelopes => {
          if (envelopes.length === 0) {
            return Observable.of({ ...current })
          }

          // We only care about events we ourselves are actually subscribed to.
          const filtered = envelopes.filter(eventFilter)
          if (filtered.length === 0) {
            return Observable.of({ ...current })
          }

          // We must wait for the final psn of our generated events
          // to be applied to the state, before we may apply the next command.
          const finalPsn = filtered[filtered.length - 1].psn

          return Observable.of({ waitFor: finalPsn })
        })
      })

    return result.pipe(
      catchError(x => {
        unblock()
        onError(x)
        return Observable.of(current)
      }),
      tap(() => {
        unblock()
        onComplete()
      }),
    )
  }

  const subscription = commandIn.mergeScan(cmdScanAcc, { waitFor: Psn.min }, 1).subscribe()

  return {
    subject: commandIn,
    subscription,
  }
}

const hydrateV2 = (
  eventStore: EventStore,
  snapshotStore: SnapshotStore,
  pondStateTracker: PondStateTracker,
) => <S, E>(
  subscriptionSet: SubscriptionSet,
  initialState: S,
  onEvent: (state: S, event: E, metadata: Metadata) => S,
  cacheKey: FishId,
  enableLocalSnapshots: boolean,
  isReset?: IsReset<E>,
): Observable<StateWithProvenance<S>> => {
  const snapshotScheduler = SnapshotScheduler.create(10)
  const semantics = cacheKey.entityType
    ? Semantics.of(cacheKey.entityType)
    : Semantics.internal('untyped-aggregation')
  const fishName = FishName.of(cacheKey.name)

  const { sourceId } = eventStore

  const metadata = (ev: Event) => ({
    isLocalEvent: ev.sourceId === sourceId,
    tags: ev.tags,
    timestampMicros: ev.timestamp,
    timestampAsDate: Timestamp.toDate.bind(null, ev.timestamp),
    lamport: ev.lamport,
  })

  // We construct a "Fish" from the given parameters in order to use the unchanged FES.
  const info: FishInfo<S> = {
    semantics,
    fishName,
    initialState: () => clone(initialState),
    subscriptionSet,

    onEvent: (state, ev) => onEvent(state, ev.payload as E, metadata(ev)),

    isSemanticSnapshot: isReset ? (ev: Event) => isReset(ev.payload as E, metadata(ev)) : undefined,

    // TODO proper support
    snapshotFormat: enableLocalSnapshots
      ? SnapshotFormat.identity(cacheKey.version || 0)
      : undefined,
  }

  return eventStore
    .present()
    .take(1)
    .concatMap(present => {
      const init = FishEventStore.initialize(
        info,
        eventStore,
        snapshotStore,
        snapshotScheduler,
        present.psns,
      )

      return init.map(fes => ({ fes, present }))
    })
    .concatMap(({ present, fes }) => {
      const liveEvents = eventStore
        .allEvents(
          {
            psns: present.psns,
            default: 'min',
          },
          { psns: {}, default: 'max' },
          subscriptionSet,
          AllEventsSortOrders.Unsorted,
          // EventKey.zero, // optional
        )
        .concatMap(intoOrderedChunks)
        .filter(evs => evs.length > 0)

      const mergeScanSeed: EventScanState<S, E> = {
        eventStore: fes,
        emit: [],
      }

      const accumulator = mkEventScanAcc<S, E>(pondStateTracker, semantics, fishName)

      return Observable.concat(
        fes.currentState().take(1),
        liveEvents.mergeScan(accumulator, mergeScanSeed, 1).concatMap(x => x.emit),
      )
    })
}

export const FishJar = {
  hydrateV2,
  commandPipeline,
}
