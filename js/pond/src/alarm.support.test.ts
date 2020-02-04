import { Subscription } from './subscription'
import {
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  OnStateChange,
  Semantics,
  Source,
  Timestamp,
} from './types'
import { unreachableOrElse } from './util/'
import { Opaque } from './util/opaqueTag'
export declare const SequenceNumberTag: unique symbol
export type SequenceNumber = Opaque<number, typeof SequenceNumberTag>
const mkSequenceNumber = (sequence: number): SequenceNumber => sequence as SequenceNumber
export const SequenceNumber = {
  of: mkSequenceNumber,
  zero: mkSequenceNumber(0),
  incr: (seq: SequenceNumber) => SequenceNumber.of(seq + 1),
}

export type AlarmEvent = {
  type: 'alarmRaised'
  sequence: SequenceNumber
  timestamp: Timestamp
  source: Source
  message: string
}

export type Event = AlarmEvent | { type: 'alarmAcknowledged'; sequence: SequenceNumber }

export type AcknowledgeAlarm = { type: 'acknowledgeAlarm'; sequence: SequenceNumber }

export type Command =
  | { type: 'registerAlarm'; timestamp: Timestamp; source: Source; message: string }
  | AcknowledgeAlarm

export type UIState = { open: AlarmEvent[] }

export type State = { sequence: SequenceNumber } & UIState

export const project: (state: State) => UIState = state => ({ open: state.open })

const initialState: InitialState<State> = () => ({
  state: { sequence: SequenceNumber.zero, open: [] },

  subscriptions: [Subscription.of(alarmFishType)],
})

const onEvent: OnEvent<State, Event> = (state, event) => {
  const p = event.payload
  switch (p.type) {
    case 'alarmRaised': {
      const open = state.open.slice(0)
      open.push(p)
      return { sequence: SequenceNumber.incr(state.sequence), open }
    }
    case 'alarmAcknowledged': {
      const open = state.open.filter(e => e.sequence !== p.sequence)
      return { ...state, open }
    }
    default:
      return unreachableOrElse(p, state)
  }
}

const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'registerAlarm': {
      const { sequence } = state
      const { timestamp, source, message } = command
      return [{ type: 'alarmRaised', sequence, timestamp, source, message }]
    }
    case 'acknowledgeAlarm': {
      const { sequence } = command
      if (state.open.find(a => a.sequence === sequence)) {
        return [{ type: 'alarmAcknowledged', sequence }]
      }
      return []
    }
    default:
      return unreachableOrElse(command, [])
  }
}

export const alarmFishType = FishType.of<State, Command, Event, UIState>({
  semantics: Semantics.jelly('Alarm'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: OnStateChange.publishState(project),
})
