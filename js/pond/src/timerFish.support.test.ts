// a timer fish to test pond infrastructure
import { Observable } from 'rxjs'
import { gen as cgen, Generator } from 'testcheck'
import { unreachableOrElse } from '.'
import {
  Envelope,
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  OnStateChange,
  Semantics,
  StateEffect,
} from './types'
export { Generator } from 'testcheck'

export type State = { type: 'initial' } | { type: 'disabled' } | { type: 'enabled'; ping: number }

export type Command =
  | { type: 'enable' }
  | { type: 'disable' }
  | { type: 'ping' }
  | { type: 'reset' }

export type Event =
  | { type: 'reset' }
  | { type: 'enabled' }
  | { type: 'disabled' }
  | { type: 'pinged' }

const events: Event[] = [
  { type: 'reset' },
  { type: 'enabled' },
  { type: 'disabled' },
  { type: 'pinged' },
]
const commands: Command[] = [
  { type: 'reset' },
  { type: 'enable' },
  { type: 'disable' },
  { type: 'ping' },
]
export type Generators = {
  event: Generator<Event>
  command: Generator<Command>
}
export const gen: Generators = {
  event: cgen.oneOf(events),
  command: cgen.oneOf(commands),
}

function mkTimer(): Observable<StateEffect<Command, State>> {
  return Observable.defer(() =>
    Observable.timer(100).map<number, StateEffect<Command, State>>(() =>
      StateEffect.sendSelf({ type: 'ping' }),
    ),
  )
}

// crucial: needs both commands, see pond.spec.js
function mkReset(): Observable<StateEffect<Command, State>> {
  return Observable.of<Command>({ type: 'reset' }, { type: 'enable' }).map(StateEffect.sendSelf)
}

const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  switch (event.payload.type) {
    case 'enabled':
      return { type: 'enabled', ping: 0 }
    case 'disabled':
      return { type: 'disabled' }
    case 'pinged':
      return state.type === 'enabled' ? { type: 'enabled', ping: state.ping + 1 } : state
    case 'reset':
      return { type: 'initial' }
    default:
      return unreachableOrElse(event.payload, state)
  }
}

const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'enable':
      return state.type === 'initial' ? [{ type: 'enabled' }] : []
    case 'disable':
      return state.type === 'enabled' ? [{ type: 'disabled' }] : []
    case 'ping':
      return state.type === 'enabled' ? [{ type: 'pinged' }] : []
    case 'reset':
      return [{ type: 'reset' }]
    default:
      return unreachableOrElse(command, [])
  }
}

const getCommandEffect = (t: string) => {
  switch (t) {
    case 'enabled':
      return mkTimer()
    case 'disabled':
      return mkReset()
    default:
      return Observable.empty<StateEffect<Command, State>>()
  }
}

const onStateChange: OnStateChange<State, Command, State> = pond =>
  pond.observeSelf().switchMap(state => {
    return Observable.concat(
      Observable.of(StateEffect.publish(state)),
      getCommandEffect(state.type),
    )
  })

const initialState: InitialState<State> = () => ({ state: { type: 'initial' } })

export const timerFishType: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>({
  semantics: Semantics.of('timerFish'),
  initialState,
  onEvent,
  onCommand,
  onStateChange,
})

export const brokenTimerFishType: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>({
  semantics: Semantics.of('timerFish'),
  initialState,
  onEvent,
  onCommand: () => {
    throw new Error('I am broken!')
  },
  onStateChange,
})
