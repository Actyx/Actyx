/* eslint-disable @typescript-eslint/ban-ts-ignore */

import { CommandApi } from '.'
import { mockArticles } from './mockdata.support.test'
import { Subscription } from './subscription'
import {
  FishName,
  FishType,
  FishTypeImpl,
  OnCommand,
  OnStateChange,
  Semantics,
  SourceId,
  Target,
} from './types'
import { unreachableOrElse } from './util/'

type Stats = {
  productionTime: number
  pauseTime: number
  produced: number
  scrap: number
}

type Documentation = {
  readonly location: string
  readonly page?: number
}

type WorkProcess = {
  readonly workstationId: string
  readonly instructions: string[]
  readonly documentation?: Documentation
  readonly totals: Stats
}

export type ArticleConfig = {
  readonly id: string
  readonly description: string
  readonly billOfMaterials: { article: string; quantity: number }[]
  readonly steps: WorkProcess[]
  readonly documentationLink: string
}

type ConflictElement = { sourceId: SourceId; workstation: string }

type ProcessingState = {
  readonly state: 'running' | 'paused'
  readonly workstation: string
  readonly sourceId: SourceId
}

type ConflictState = {
  readonly state: 'conflict'
  readonly workstations: ConflictElement[]
}

type WorkState = ConflictState | ProcessingState

type StepState = {
  readonly lastStateChange: number // Microseconds since Unix epoch. Date.now() * 1000
  readonly currentState: WorkState
  readonly stepStats: Stats
}

export type State = {
  readonly config: ArticleConfig
  readonly currentSteps: StepState[] // Indexed by step number
}

export type ConfigReply = {
  readonly type: 'configReply'
  readonly config: ArticleConfig
}

export type CommandGetConfig = {
  readonly type: 'getConfig'
  readonly replyTo: Target<ConfigReply>
}
export type Command = CommandGetConfig

type Event = undefined

export const mkConflict = (workstations: ConflictElement[]): WorkState => ({
  state: 'conflict',
  workstations,
})

const initialState = (name: string) => {
  const a = mockArticles.find((art: ArticleConfig) => art.id === name)
  if (a) {
    return {
      state: {
        config: a,
        currentSteps: [],
      },

      subscriptions: [Subscription.of(articleFishType, FishName.of(name))],
    }
  }
  throw new Error(`nonexistent article ${name}`)
}

const { pond } = CommandApi

const events = (...es: Event[]): ReadonlyArray<Event> => es
const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'getConfig': {
      const reply: ConfigReply = { type: 'configReply', config: state.config }
      return pond.send(command.replyTo)(reply)
    }

    default:
      return unreachableOrElse(command.type, events())
  }
}

export const articleFishType: FishTypeImpl<State, Command, Event, State> = FishType.of({
  semantics: Semantics.of('article'),
  initialState,
  onCommand,
  onEvent: (state: State, _whatever) => state,
  onStateChange: OnStateChange.publishPrivateState(),
})
