// a command probe fish
import { CommandApi, FishType, InitialState, OnCommand, OnEvent, Semantics } from '..'
import { OnStateChange } from '../types'

export type State = unknown

export type Command = unknown

export type Event = unknown

const onEvent: OnEvent<State, Event> = (_state, event) => event.payload

const onCommand: OnCommand<State, Command, Event> = (_, command) => CommandApi.of([command])

const initialState: InitialState<State> = () => ({
  state: null,
})

export const commandProbe: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>({
  semantics: Semantics.of('probe'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: OnStateChange.publishPrivateState(),
})
