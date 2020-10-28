import { Event, Events, OffsetMap } from '../eventstore/types'
import { StateWithProvenance } from '../types'

export type Reducer<S> = {
  appendEvents: (events: Events) => StateWithProvenance<S>

  setState: (state: StateWithProvenance<S>) => void
}

export const MonotonicReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: StateWithProvenance<S>,
): Reducer<S> => {
  let swp = initialState

  return {
    appendEvents: (events: Events) => {
      let { state, psnMap } = swp

      for (const ev of events) {
        state = onEvent(state, ev)
        psnMap = OffsetMap.update(psnMap, ev)
      }

      swp = { state, psnMap }

      return swp
    },

    setState: s => (swp = s),
  }
}
