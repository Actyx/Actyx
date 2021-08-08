/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  ActyxEvent,
  EventFns,
  Metadata,
  Milliseconds,
  Offset,
  OffsetMap,
  StateWithProvenance,
  StreamId,
  Timestamp,
  Where,
} from '@actyx/sdk'
import { lessThan } from 'fp-ts/lib/Ord'
import { Map } from 'immutable'
import { Observable, Subject, Subscription as RxSubscription } from 'rxjs'
import { catchError, tap } from 'rxjs/operators'
import log from './loggers'
import { PondStateTracker } from './pond-state'
import { Fish, FishId } from './types'
import { lookup } from './util'

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

export type CommandPipeline<S, I> = Readonly<{
  // Subject where new commands must be pushed
  subject: Subject<CommandInput<S, I>>

  // Subscription to the running pipeline (cancel to destroy pipeline)
  subscription: RxSubscription
}>

type CommandScanState = Readonly<{
  waitFor: Record<StreamId, Offset>
}>

const hasAllOffsets = (offsets: Record<StreamId, Offset>, waitFor: Record<StreamId, Offset>) => {
  for (const [stream, offsetToWaitFor] of Object.entries(waitFor)) {
    const latestSeen = lookup(offsets, stream)
    if (latestSeen === undefined || offsetToWaitFor > latestSeen) {
      return false
    }
  }

  return true
}

const commandPipeline = <S, I>(
  pondStateTracker: PondStateTracker,
  semantics: string,
  name: string,
  handler: ((input: I) => Promise<Metadata[]>),
  stateSubject: Observable<StateWithProvenance<S>>,
  eventFilter: ((t: Metadata) => boolean),
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
        const pass = hasAllOffsets(stateWithProvenance.offsets, current.waitFor)

        if (!pass) {
          log.pond.debug(
            semantics,
            '/',
            name,
            ' waiting for',
            JSON.stringify(current.waitFor),
            '; currently at:',
            JSON.stringify(stateWithProvenance.offsets),
          )
        }
        return pass
      })
      .map(sp => sp.state)
      .take(1)
      .concatMap(s => {
        const onCommandResult = command(s)
        const stored = Observable.from(handler(onCommandResult))

        return stored.concatMap(envelopes => {
          if (envelopes.length === 0) {
            return Observable.of({ ...current })
          }

          // We only care about events we ourselves are actually subscribed to.
          const filtered = envelopes.filter(eventFilter)
          if (filtered.length === 0) {
            return Observable.of({ ...current })
          }

          // We must wait for all of our generated events to be applied to the state,
          // before we may apply the next command.
          // The events may be in different streams.
          const finalOffsets: Record<StreamId, Offset> = {}

          for (const env of envelopes) {
            // Since envelopes are returned in ascending order, we will
            // just overwrite the same entry again and again with higher number
            // in case the events all go into the same stream.
            finalOffsets[env.stream] = env.offset
          }

          return Observable.of({ waitFor: finalOffsets })
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

  const subscription = commandIn.mergeScan(cmdScanAcc, { waitFor: {} }, 1).subscribe()

  return {
    subject: commandIn,
    subscription,
  }
}

const getEventsForwardChunked = (
  fns: EventFns,
  subscriptionSet: Where<unknown>,
  present: OffsetMap,
): Observable<ActyxEvent[]> =>
  new Observable<ActyxEvent[]>(o =>
    fns.queryKnownRangeChunked(
      {
        upperBound: present,
        query: subscriptionSet,
      },
      500,
      chunk => o.next(chunk.events),
      () => o.complete(),
    ),
  )

type StartedFish<S> = {
  fish: Fish<S, any>
  startedFrom: ActyxEvent
}

export type StartedFishMap<S> = Map<string, StartedFish<S>>

const observeAll = (eventStore: EventFns, _pondStateTracker: PondStateTracker) => <ESeed, S>(
  firstEvents: Where<ESeed>,
  makeFish: (seed: ESeed) => Fish<S, any> | undefined,
  expireAfterSeed?: Milliseconds,
): Observable<StartedFishMap<S>> => {
  const fish$ = Observable.from(eventStore.present()).concatMap(present => {
    const persisted = getEventsForwardChunked(eventStore, firstEvents, present)

    // This step is only so that we don’t emit outdated collection while receiving chunks of old events
    const initialFishs = persisted.reduce((acc: Record<string, StartedFish<S>>, chunk) => {
      for (const evt of chunk) {
        const fish = makeFish(evt.payload as ESeed)

        if (fish !== undefined) {
          acc[FishId.canonical(fish.fishId)] = { fish, startedFrom: evt }
        }
      }
      return acc
    }, {})

    return initialFishs.concatMap(
      observeAllStartWithInitial(eventStore, makeFish, firstEvents, present, expireAfterSeed),
    )
  })

  return fish$
}

const earlier = lessThan(ActyxEvent.ord)

const mkPrune = (timeout?: Milliseconds) => {
  if (!timeout) return <S>(cur: Map<string, StartedFish<S>>) => cur

  const timeoutMicros = Milliseconds.toTimestamp(timeout)

  return <S>(cur: Map<string, StartedFish<S>>) => {
    const now = Timestamp.now()
    return cur.filter(started => started.startedFrom.meta.timestampMicros + timeoutMicros > now)
  }
}

const observeAllStartWithInitial = <ESeed, S>(
  eventStore: EventFns,
  makeFish: (seed: ESeed) => Fish<S, any> | undefined,
  subscriptionSet: Where<unknown>,
  present: OffsetMap,
  expireAfterSeed?: Milliseconds,
) => (init: Record<string, StartedFish<S>>) => {
  // Switch to immutable representation so as to not screw over downstream consumers
  let immutableFishSet = Map(init)

  const liveEvents = new Observable<ActyxEvent[]>(o =>
    eventStore.subscribeChunked(
      {
        lowerBound: present,
        query: subscriptionSet,
      },
      {
        // Buffer slightly to improve performance
        maxChunkTimeMs: 20,
      },
      chunk => o.next(chunk.events),
    ),
  )

  const prune = mkPrune(expireAfterSeed)

  const updates = liveEvents.concatMap(chunk => {
    const oldSize = immutableFishSet.size

    for (const evt of chunk) {
      const fish = makeFish(evt.payload as ESeed)

      if (fish === undefined) {
        continue
      }

      const newEntry = { fish, startedFrom: evt }

      // Latest writer wins. This is only relevant for expiry -- the Fish ought to be the same, and the Pond will have it cached.
      immutableFishSet = immutableFishSet.update(
        FishId.canonical(fish.fishId),
        existing => (!existing || earlier(existing.startedFrom, evt) ? newEntry : existing),
      )
    }

    const newSize = immutableFishSet.size
    const newFishAppeared = newSize > oldSize

    immutableFishSet = prune(immutableFishSet)
    const oldFishPruned = immutableFishSet.size < newSize

    if (newFishAppeared || oldFishPruned) {
      return [immutableFishSet]
    }

    return []
  })

  return updates.startWith(Map(init))
}

export const FishJar = {
  commandPipeline,
  observeAll,
}
