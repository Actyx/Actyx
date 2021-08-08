import { ActyxEvent, LocalSnapshot, OffsetMap } from '@actyx/sdk'
import { clone } from 'ramda'
import { SimpleReducer } from './types'

export const simpleReducer = <S>(
  onEvent: (oldState: S, event: ActyxEvent) => S,
  initialState: LocalSnapshot<S>,
  isReset: (event: ActyxEvent) => boolean,
): SimpleReducer<S> => {
  const backupState = clone(initialState.state)

  // Head is always the latest state known to us
  let head: LocalSnapshot<S> = initialState

  // Advance the head by applying the given event array between (i ..= iToInclusive)
  // without modifying the existing head; WILL modify the `state` inside `head`, though!
  // State is serialized upstream by cachingReducer, hence later modifications are OK.
  const appendEvents = (events: ActyxEvent[], fromIdx: number, toIdxInclusive: number) => {
    if (fromIdx > toIdxInclusive) {
      return head
    }

    let i = fromIdx

    let { state, eventKey, cycle, horizon } = head
    const offsets = { ...head.offsets }

    while (i <= toIdxInclusive) {
      const ev = events[i]

      if (isReset(ev)) {
        horizon = ev.meta
        state = clone(backupState)
        cycle = 0
      }

      state = onEvent(state, ev)
      OffsetMap.update(offsets, ev.meta)
      eventKey = ev.meta

      i += 1
      cycle += 1
    }

    head = {
      state,
      offsets: offsets,
      cycle,
      eventKey,
      horizon,
    }

    return head
  }

  const setState = (snap: LocalSnapshot<S>) => {
    head = snap
  }

  return {
    appendEvents,
    setState,
  }
}
