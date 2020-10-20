/* eslint-disable @typescript-eslint/no-explicit-any */
import { clone } from 'ramda'
import { Observable, Observer, ReplaySubject, Subject, Scheduler } from 'rxjs'
import { Event, EventStore, OffsetMap } from '../eventstore'
import { PondStateTracker } from '../pond-state'
import { SnapshotStore } from '../snapshotStore'
import { SubscriptionSet } from '../subscription'
import { FishId, IsReset, Metadata, SourceId, StateWithProvenance, toMetadata } from '../types'
import { eventsMonotonic, MsgType, EventsOrTimetravel } from './endpoint'
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

        const initReducer = () => MonotonicReducer(onEventRaw, { state: clone(initialState), psnMap: {} })
        let reducer = initReducer()

        const out: Subject<StateWithProvenance<S>> = new ReplaySubject(1)


        const observer: Observer<EventsOrTimetravel> = {
            next: (msg) => {
                switch (msg.type) {
                    case MsgType.state:
                        reducer.setState(msg.state as StateWithProvenance<S>)
                        return

                    case MsgType.events: {
                        // TODO: Store snapshots (must be async into some pipeline)
                        // TODO: caughtUp handling
                        out.next(reducer.appendEvents(msg.events))
                        return
                    }

                    case MsgType.timetravel: {
                        // TODO: Find locally cached state
                        reducer = initReducer()
                        return
                    }
                }
            },

            error: (e) => {
                console.log(e)
            },

            complete: () => {
                // TODO: Try loading snapshot (wait for snapshot pipeline -> then load)
                const current = reducer.currentOffsets()
                startFromScratch(OffsetMap.isEmpty(current) ? undefined : current)
            }
        }

        const startFromScratch = (from?: OffsetMap): void => {
            endpoint(subscriptionSet, from).subscribeOn(Scheduler.queue).subscribe(observer)
        }

        // This could be phrased in terms of a mergeScan, but there’d be no gain in clarity imho.
        startFromScratch()

        return out
    }
