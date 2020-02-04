import { lensPath, set } from 'ramda'
import { CommandAsync, unreachableOrElse } from '..'
import { FishType, OnCommand, OnEvent, OnStateChange, Semantics, Timestamp } from '../types'

export const enum PowerCondition {
  on = 'on',
  off = 'off',
  running = 'running',
  na = 'na',
}
export enum EventType {
  counterSet = 'counterSet',
  powerConditionSet = 'powerConditionSet',
}

export type Event =
  | { type: EventType.counterSet; id: string; reading: number; timestamp: Timestamp }
  | { type: EventType.powerConditionSet; condition: PowerCondition; timestamp: Timestamp }

export const enum CommandType {
  inject = 'inject',
}
export type Command = { type: 'inject'; ev: Event }

export type CounterReading = { reading: number; timestamp: Timestamp }

export type State = {
  counters: { [_: string]: CounterReading }
  powerCondition: { condition: PowerCondition; timestamp: Timestamp }
}

const onCommand: OnCommand<State, Command, Event> = (_state, command) => {
  switch (command.type) {
    case CommandType.inject: {
      return CommandAsync.of([command.ev])
    }
    default:
      return CommandAsync.noEvents
  }
}

const onEvent: OnEvent<State, Event> = (state, event) => {
  const { payload } = event
  switch (payload.type) {
    case EventType.powerConditionSet: {
      const { condition, timestamp } = payload
      return { ...state, powerCondition: { condition, timestamp } }
    }
    case EventType.counterSet: {
      const { id, reading, timestamp } = payload
      const lens = lensPath<CounterReading, State>(['counters', id])
      const state0: State = set<State>(lens, { reading, timestamp })(state)
      return state0
    }
    default:
      return unreachableOrElse(payload, state)
  }
}

export const enum ValidationFailure {
  invalidPayload = 'invalidPayload',
}

export const genericMachineFish = {
  type: FishType.of<State, Command, Event, State>({
    semantics: Semantics.of('iot-generic-machine'),
    initialState: () => ({
      state: {
        counters: {},
        powerCondition: { condition: PowerCondition.na, timestamp: Timestamp.zero },
      },
    }),
    onEvent,
    onCommand,
    onStateChange: OnStateChange.publishPrivateState(),
  }),
}
