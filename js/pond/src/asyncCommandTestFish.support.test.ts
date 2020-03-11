/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
// a fish to test async command processing
import { Target } from '.'
import { CommandApi } from './commandApi'
import {
  AsyncCommandResult,
  Envelope,
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  OnStateChange,
  Semantics,
} from './types'

export type State = {
  hashes: { [key: string]: string }
}

export type SlowCommand = {
  type: 'slow'
  id: number
  target: Target<any>
}

export type Command =
  | {
      type: 'navigate'
      path: string
    }
  | {
      type: 'send'
      target: Target<any>
      cmd: any
    }
  | { type: 'broken' }
  | SlowCommand

export const enum EventType {
  Imported = 'imported',
  Slow = 'slow',
}

export type Event =
  | {
      type: EventType.Imported
      url: string
      hash: string
    }
  | {
      type: EventType.Slow
      count: number
    }

const onEvent: OnEvent<State, Event> = (state: State, event: Envelope<Event>) => {
  switch (event.payload.type) {
    case EventType.Imported: {
      const { hash, url } = event.payload
      // ramda?
      const hashes = { ...state.hashes }

      hashes[url] = hash
      return { hashes }
    }
    case EventType.Slow: {
      return state
    }
  }
}

const { noEvents, http, pond } = CommandApi

const slowCommand = (cmd: SlowCommand): AsyncCommandResult<Event> =>
  noEvents
    .chain(() => pond.send(cmd.target)(`starting ${cmd.id}`))
    .chain(() => http.get('http://slow.com'))
    .chain(() => pond.send(cmd.target)(`ending ${cmd.id}`))

const onCommand: OnCommand<State, Command, Event> = (_, command) => {
  switch (command.type) {
    case 'send':
      return pond.send(command.target)(command.cmd)
    case 'slow':
      return slowCommand(command)
    case 'broken':
      return undefined as any
  }
}

const initialState: InitialState<State> = () => ({
  state: { hashes: {} },
})

export const asyncTestFish = FishType.of<State, Command, Event, State>({
  semantics: Semantics.of('asyncTestFish'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: OnStateChange.publishPrivateState(),
})
