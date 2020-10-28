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
    LocalSnapshot,
} from '../types'
import { eventsMonotonic, EventsOrTimetravel, MsgType, StateMsg, EventsMsg } from './endpoint'
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
        const findInitialState = (_before: EventKey): LocalSnapshot<S> => ({
            state: clone(initialState),
            psnMap: {},
            cycle: 0,
            eventKey: EventKey.zero,
            horizon: undefined
        })

        const reducer = MonotonicReducer(onEventRaw, findInitialState(EventKey.zero))

        const startFromScratch = (from?: OffsetMap): Observable<EventsOrTimetravel> => {
            const restart = Observable.defer(() => {
                const lastGoodState: StateWithProvenance<S> = findInitialState(EventKey.zero)
                const startFrom = lastGoodState.psnMap
                return startFromScratch(OffsetMap.isEmpty(startFrom) ? undefined : startFrom)
            })

            const start = endpoint(subscriptionSet, from)
                .subscribeOn(Scheduler.queue)
                .catch(err => {
                    console.log(err)
                    // Just continue into the concatenated "restart" Observable
                    return Observable.from([])
                })

            return Observable.concat(start, restart)
        }

        // On time travel, switch to a fresh subscribeMonotonic stream
        const updates = (from?: OffsetMap): Observable<StateMsg | EventsMsg> => endpoint(subscriptionSet, from).subscribeOn(Scheduler.queue)
            .concatMap(msg => {
                if (msg.type === MsgType.timetravel) {
                    const latestValid = findInitialState(msg.trigger)
                    const resetMsg: StateMsg = {
                        type: MsgType.state,
                        state: latestValid
                    }

                    const startFrom = latestValid.psnMap

                    return Observable.concat(
                        Observable.of(resetMsg),
                        updates(OffsetMap.isEmpty(startFrom) ? undefined : startFrom)
                    )
                } else {
                    return [msg]
                }
            })

        const updates$ = updates()

        return updates$.concatMap(msg => {
            switch (msg.type) {
                // case MsgType.timetravel: {
                //     reducer.setState(findInitialState(msg.trigger))
                //     return []
                // }

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
