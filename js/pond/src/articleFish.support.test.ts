/* eslint-disable @typescript-eslint/ban-ts-ignore */

import { append, clone, ifElse, isNil, lensPath, lensProp, over, pipe, set, view } from 'ramda'
import { CommandApi } from '.'
import { mockArticles } from './mockdata.support.test'
import { Subscription } from './subscription'
import {
  FishName,
  FishType,
  FishTypeImpl,
  OnCommand,
  OnEvent,
  OnStateChange,
  Semantics,
  SourceId,
  Target,
} from './types'
import { unreachableOrElse } from './util/'

export type Rejection = {
  readonly type: 'rejected'
  readonly command: object
  readonly error: string
}

export type Stats = {
  productionTime: number
  pauseTime: number
  produced: number
  scrap: number
}

export type Documentation = {
  readonly location: string
  readonly page?: number
}

export type WorkProcess = {
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

export type ConflictElement = { sourceId: SourceId; workstation: string }

export type ProcessingState = {
  readonly state: 'running' | 'paused'
  readonly workstation: string
  readonly sourceId: SourceId
}

export type ConflictState = {
  readonly state: 'conflict'
  readonly workstations: ConflictElement[]
}

export type WorkState = ConflictState | ProcessingState

export type StepState = {
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
export type CommandStartStep = {
  readonly type: 'startStep'
  readonly step: number
  readonly workstation: string
  readonly replyTo?: Target<Rejection>
}
export type CommandPauseStep = {
  readonly type: 'pauseStep'
  readonly step: number
  readonly workstation: string
  readonly replyTo?: Target<Rejection>
}
export type CommandFinishStep = {
  readonly type: 'finishStep'
  readonly step: number
  readonly workstation: string
  readonly quantity: number
  readonly scrap: number
  readonly replyTo?: Target<Rejection>
}
export type CommandCancelStep = {
  readonly type: 'cancelStep'
  readonly step: number
  readonly workstation: string
  readonly replyTo?: Target<Rejection>
}
export type Command =
  | CommandGetConfig
  | CommandStartStep
  | CommandPauseStep
  | CommandFinishStep
  | CommandCancelStep

export type EventWorkStarted = {
  readonly type: 'workStarted'
  readonly step: number
  readonly workstation: string
}
export type EventWorkPaused = {
  readonly type: 'workPaused'
  readonly step: number
  readonly workstation: string
}
export type EventWorkUnpaused = {
  readonly type: 'workUnpaused'
  readonly step: number
  readonly workstation: string
}
export type EventWorkFinished = {
  readonly type: 'workFinished'
  readonly step: number
  readonly workstation: string
  readonly quantity: number
  readonly scrap: number
}
export type EventWorkCancelled = {
  readonly type: 'workCancelled'
  readonly step: number
  readonly workstation: string
}
export type Event =
  | EventWorkStarted
  | EventWorkPaused
  | EventWorkUnpaused
  | EventWorkFinished
  | EventWorkCancelled

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

const onEvent: OnEvent<State, Event> = (state, event) => {
  const payload = event.payload

  /**
   * Handles the current step, if it's known. Otherwise returns s0.
   * @param stepHandler StepState => {state: State} Function to calculate the new state.
   * @param s0 State
   * @param stepCount Current step
   */
  const handleCurrentStepIfPresent = (
    stepHandler: (stepState: StepState) => State,
    s0: State,
    stepCount: number,
  ): State => ifElse(isNil, () => s0, stepHandler)(s0.currentSteps[stepCount])

  const evConflict: ConflictElement = {
    sourceId: event.source.sourceId,
    workstation: payload.workstation,
  }

  const mkConflictState = (c: WorkState): WorkState => {
    // This should be only necessary for flow
    if (c.state !== 'conflict') {
      const { sourceId, workstation } = c
      return mkConflict([{ sourceId, workstation }, evConflict])
    }
    return c
  }

  /**
   * Verifies the current workstation and returns the appropriate StepState.
   * @param {WorkState} currentState
   * @param {Stats} stepStats
   * @param {'running' | 'paused'} newState
   */
  const verifyWorkstation = (
    currentState: WorkState,
    stepStats: Stats,
    newState: 'running' | 'paused',
  ): StepState => {
    if (currentState.state === 'conflict' || currentState.workstation !== payload.workstation) {
      return {
        lastStateChange: event.timestamp,
        currentState: mkConflictState(currentState),
        stepStats,
      }
    }
    return {
      lastStateChange: event.timestamp,
      currentState: {
        state: newState,
        workstation: currentState.workstation,
        sourceId: event.source.sourceId,
      },
      stepStats,
    }
  }

  /**
   * Create the a newly started processing step.
   * @param initial State to create the step with ('running' | 'paused')
   */
  const createStep = (initial: 'running' | 'paused'): StepState => {
    const stepStats = { productionTime: 0, pauseTime: 0, produced: 0, scrap: 0 }
    return {
      lastStateChange: event.timestamp,
      currentState: {
        state: initial,
        workstation: payload.workstation,
        sourceId: event.source.sourceId,
      },
      stepStats,
    }
  }

  /**
   * Check, if a given conflict is already known.
   * @param {ConflictState} cs0 Current StepState
   */
  const isConflictKnown = (cs0: ConflictState): boolean =>
    cs0.workstations.find(
      w => w.sourceId === evConflict.sourceId && w.workstation === evConflict.workstation,
    ) !== undefined

  /**
   *  Checks for potential conflicts. If no conflicts exist or arose, execute stepHandler.
   * @param cs0 Current StepState
   * @param stepHandler Function to calculate the resulting StepState, if no conflict is detected.
   */
  const checkConflict = (
    cs0: StepState,
    stepHandler: (stepState: StepState) => StepState,
  ): StepState => {
    const currentState = cs0.currentState
    if (currentState.state === 'conflict') {
      if (isConflictKnown(currentState)) {
        return stepHandler(cs0)
      }
      // @ts-ignore
      const cs = over(lensPath(['currentState', 'workstations']), append(evConflict))(cs0)
      return stepHandler(cs)
    }
    return stepHandler(cs0)
  }

  /**
   * Handles incoming work events (started, paused and unpaused) and calculates the new StepState.
   * @param s0 State
   * @param e Event to handle
   * @param stepHandler: Function to calculate the resulting StepState,
   *        if no conflict is detected.
   */
  const handleWorkingStep = (
    s0: State,
    e: EventWorkStarted | EventWorkPaused | EventWorkUnpaused,
    stepHandler: (stepState: StepState) => StepState,
  ): State =>
    // @ts-ignore
    over(
      lensPath(['currentSteps', e.step]),
      ifElse(
        isNil,
        () => createStep(e.type === 'workPaused' ? 'paused' : 'running'),
        (step: StepState) => checkConflict(step, stepHandler),
      ),
    )(s0)

  /**
   * Calculate the final stats for a given step.
   * @param initial Initial stats
   * @param cs Current Step
   * @param eventTs Timestamp of finished event
   * @param qty Produced quantity
   * @param scrap Produced scrap
   */
  const calcJobSummary = (
    initial: Stats,
    cs: StepState,
    eventTs: number,
    qty: number,
    scrap: number,
  ): Stats => {
    const stats = clone(initial)
    const elapsed = eventTs - cs.lastStateChange
    if (cs.currentState.state === 'running') {
      stats.productionTime += elapsed
    } else {
      stats.pauseTime += elapsed
    }
    stats.productionTime += cs.stepStats.productionTime
    stats.pauseTime += cs.stepStats.pauseTime
    stats.produced += qty
    stats.scrap += scrap
    return stats
  }

  switch (payload.type) {
    case 'workStarted': {
      // Create the step. If it's already there, go to conflict state.
      return handleWorkingStep(state, payload, over(lensProp('currentState'), mkConflictState))
    }
    case 'workPaused': {
      // Create the step (if not yet there). Otherwise, calculate stats.
      return handleWorkingStep(state, payload, (x: StepState) => {
        const elapsed = event.timestamp - x.lastStateChange
        const productionTime = x.stepStats.productionTime + elapsed
        const stepStats = { ...x.stepStats, productionTime }
        return verifyWorkstation(x.currentState, stepStats, 'paused')
      })
    }
    case 'workUnpaused': {
      // Create the step (if not yet there). Otherwise, calculate stats.
      return handleWorkingStep(state, payload, (x: StepState) => {
        const elapsed = event.timestamp - x.lastStateChange
        const pauseTime = x.stepStats.pauseTime + elapsed
        const stepStats = { ...x.stepStats, pauseTime }
        return verifyWorkstation(x.currentState, stepStats, 'running')
      })
    }

    case 'workFinished': {
      const finishJob = (cs: StepState): { cs?: StepState; stats: Stats } => {
        const currentState = cs.currentState
        const stats = view(lensPath(['config', 'steps', payload.step, 'totals']))(state) as Stats
        if (currentState.state === 'conflict') {
          // TODO statistics are probably not being correctly done in case of conflict
          if (isConflictKnown(currentState)) {
            return { cs, stats }
          }

          return {
            // @ts-ignore
            cs: over(lensPath(['currentState', 'workstations']), append(evConflict))(cs),
            stats,
          }
        } else if (currentState.workstation === payload.workstation) {
          return {
            cs: undefined, // Delete current step
            stats: calcJobSummary(stats, cs, event.timestamp, payload.quantity, payload.scrap),
          }
        }
        // @ts-ignore
        return { cs: over(lensProp('currentState'), mkConflictState)(cs), stats }
      }

      const handleFinishedStep = pipe(
        finishJob,
        ({ cs, stats }) => ({
          config: set(lensPath(['steps', payload.step, 'totals']), stats)(state.config),
          currentSteps: set(lensPath([payload.step]), cs)(state.currentSteps),
        }),
      )

      // @ts-ignore
      return handleCurrentStepIfPresent(handleFinishedStep, state, payload.step)
    }

    case 'workCancelled': {
      const handleCancelled = (cs: StepState): StepState | undefined => {
        const currentState = cs.currentState
        if (currentState.state === 'conflict') {
          const ws = currentState.workstations.filter(
            w => w.workstation !== evConflict.workstation || w.sourceId !== evConflict.sourceId,
          )
          if (ws.length === 1) {
            // TODO we automatically enter the running state. Should we store each workstation's
            // old state and restore it? Or enter the paused state?
            const { sourceId, workstation } = ws[0]
            // @ts-ignore
            return set(lensProp('currentState'), { state: 'running', workstation, sourceId })(cs)
          } else if (ws.length === 0) {
            return undefined
          }
          // @ts-ignore
          return set(lensPath(['currentState', 'workstations']), ws)(cs)
        } else if (payload.workstation !== currentState.workstation) {
          // both workstations pressed Cancel at the same time
          // @ts-ignore
          return over(lensProp('currentState'), mkConflictState)(cs)
        }
        return cs
      }

      return handleCurrentStepIfPresent(
        // @ts-ignore
        (cs: StepState) =>
          set(lensPath(['currentSteps', payload.step]), handleCancelled(cs))(state),
        state,
        payload.step,
      )
    }

    default:
      return unreachableOrElse(payload, state)
  }
}

const { pond, noEvents } = CommandApi

const sendRejection = (replyTo: Target<Rejection> | undefined, command: object, error: string) =>
  replyTo ? pond.send<Rejection>(replyTo)({ type: 'rejected', command, error }) : noEvents

const events = (...es: Event[]): ReadonlyArray<Event> => es
const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'getConfig': {
      const reply: ConfigReply = { type: 'configReply', config: state.config }
      return pond.send(command.replyTo)(reply)
    }

    case 'startStep': {
      const currentStep = state.currentSteps[command.step]
      if (!currentStep) {
        return events({ type: 'workStarted', step: command.step, workstation: command.workstation })
      }
      if (currentStep.currentState.state === 'paused') {
        return events({
          type: 'workUnpaused',
          step: command.step,
          workstation: command.workstation,
        })
      }
      return sendRejection(command.replyTo, command, 'Already running').map(() => [])
    }

    case 'pauseStep': {
      const currentStep = state.currentSteps[command.step]
      const currentState = currentStep.currentState.state
      if (currentState !== 'running') {
        const reason = currentState === 'conflict' ? 'Workstation in conflict' : 'Already paused'
        return sendRejection(command.replyTo, command, reason).map(() => [])
      }
      return events({ type: 'workPaused', step: command.step, workstation: command.workstation })
    }

    case 'finishStep': {
      return events({
        type: 'workFinished',
        step: command.step,
        workstation: command.workstation,
        quantity: command.quantity,
        scrap: command.scrap,
      })
    }

    case 'cancelStep': {
      const currentStep = state.currentSteps[command.step]
      if (currentStep && currentStep.currentState.state === 'conflict') {
        return events({
          type: 'workCancelled',
          step: command.step,
          workstation: command.workstation,
        })
      }
      return sendRejection(
        command.replyTo,
        command,
        'Can only cancel when article is in conflict',
      ).map(() => [])
    }

    default:
      return unreachableOrElse(command, events())
  }
}

export const articleFishType: FishTypeImpl<State, Command, Event, State> = FishType.of({
  semantics: Semantics.of('article'),
  initialState,
  onEvent,
  onCommand,
  onStateChange: OnStateChange.publishPrivateState(),
})
