import { Event, Events, OffsetMap } from '../eventstore/types'
import { StateWithProvenance } from '../types'

export type Reducer<S> = {
  appendEvents: (events: Events) => StateWithProvenance<S>

  setState: (state: StateWithProvenance<string>) => void
}

export const MonotonicReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: StateWithProvenance<string>,
  deserializeState?: (jsonState: unknown) => S,
): Reducer<S> => {
  const deserialize = deserializeState
    ? (s: StateWithProvenance<string>) => ({ ...s, state: deserializeState(JSON.parse(s.state)) })
    : (s: StateWithProvenance<string>) => ({ ...s, state: JSON.parse(s.state) as S })

  let swp = deserialize(initialState)

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

    setState: s => {
      swp = deserialize(s)
      // Clone the input offsets, since they may not be mutable
      swp.psnMap = { ...swp.psnMap }
    },
  }
}
