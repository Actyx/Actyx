import { Event, Events, OffsetMap } from '../eventstore/types'
import { EventKey, LocalSnapshot, StateWithProvenance } from '../types'

export type States<S> = {
    latest: StateWithProvenance<S>

    snapshots?: ReadonlyArray<LocalSnapshot<S>>
}

export type Reducer<S> = {
    appendEvents: (events: Events) => States<S>

    setState: (state: StateWithProvenance<S>) => void

    // Returns the point to pick up from. `undefined` means we start from latest snapshot (if any)
    timeTravel: (trigger: EventKey) => OffsetMap | undefined
}

export const MonotonicReducer = <S>(
    onEvent: (oldState: S, event: Event) => S,
    initialState: S,
): Reducer<S> => {
    let swp: StateWithProvenance<S> = {
        state: initialState,
        psnMap: {},
    }

    return {
        appendEvents: (events: Events) => {
            let { state, psnMap } = swp

            for (const ev of events) {
                state = onEvent(state, ev)
                psnMap = OffsetMap.update(psnMap, ev)
            }

            swp = { state, psnMap }

            return {
                latest: swp,
            }
        },

        setState: s => (swp = s),

        timeTravel: () => undefined,
    }
}
