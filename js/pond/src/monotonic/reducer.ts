import { Event, Events, OffsetMap } from '../eventstore/types'
import { StateWithProvenance } from '../types'

export type Reducer<S> = {
  appendEvents: (events: Events) => StateWithProvenance<S>

  setState: (state: StateWithProvenance<string>) => void
}

export const stateWithProvenanceReducer = <S>(
  onEvent: (oldState: S, event: Event) => S,
  initialState: StateWithProvenance<S>,
  deserializeState?: (jsonState: unknown) => S,
): Reducer<S> => {
  const deserialize = deserializeState
    ? (s: string): S => deserializeState(JSON.parse(s))
    : (s: string): S => JSON.parse(s) as S

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

    setState: s => {
      const oldState = deserialize(s.state)
      // Clone the input offsets, since they may not be mutable
      swp = { psnMap: { ...s.psnMap }, state: oldState }
    },
  }
}
