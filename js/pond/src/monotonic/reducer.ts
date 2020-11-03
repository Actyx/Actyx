import { Event, Events, OffsetMap } from '../eventstore/types'
import { LocalSnapshot, StateWithProvenance } from '../types'

export type Reducer<S> = {
  appendEvents: (
    events: Events,
    emit: boolean,
  ) => {
    snapshots: LocalSnapshot<string>[]
    emit: StateWithProvenance<S>[]
  }

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

  const cloneSwp = (stateWithProvenance: StateWithProvenance<S>) => {
    const offsets = { ...stateWithProvenance.psnMap }
    const state = deserialize(JSON.stringify(swp.state))
    return {
      psnMap: offsets,
      state,
    }
  }

  let swp = cloneSwp(initialState)

  return {
    appendEvents: (events: Events, emit: boolean) => {
      let { state, psnMap } = swp

      for (const ev of events) {
        state = onEvent(state, ev)
        psnMap = OffsetMap.update(psnMap, ev)
      }

      swp = { state, psnMap }

      return {
        snapshots: [],
        // This is for all downstream consumers, so we clone.
        emit: emit ? [cloneSwp(swp)] : [],
      }
    },

    setState: s => {
      const oldState = deserialize(s.state)
      // Clone the input offsets, since they may not be mutable
      swp = { psnMap: { ...s.psnMap }, state: oldState }
    },
  }
}
