// a timer listener fish to test pond infrastructure
import { Observable } from 'rxjs'
import { timerFishType } from './timerFish.support.test'
import {
  FishName,
  FishType,
  InitialState,
  OnCommand,
  OnEvent,
  OnStateChange,
  PondObservables,
  Semantics,
  StateEffect,
} from './types'
import { unreachable } from './util/'

export type State = { type: 'disabled' } | { type: 'enabled'; count: number }

export type Command = { type: 'enable' } | { type: 'disable' } | { type: 'inc' }

export type Event = { type: 'enabled' } | { type: 'disabled' } | { type: 'inced' }

function mkTimerFishListener(
  pond: PondObservables<State>,
): Observable<StateEffect<Command, State>> {
  return pond
    .observe(timerFishType, FishName.of('nemo'))
    .filter(state => state.type === 'enabled')
    .map(state => {
      if (state.type === 'enabled') {
        return StateEffect.sendSelf<Command>({ type: 'inc' })
      }
      throw new Error('unreachable')
    })
}

const onEvent: OnEvent<State, Event> = (state, event) => {
  switch (event.payload.type) {
    case 'enabled':
      return { type: 'enabled', count: 0 }
    case 'inced':
      switch (state.type) {
        case 'enabled':
          return { type: 'enabled', count: state.count + 1 }
        default:
          return state
      }
    case 'disabled':
      return { type: 'disabled' }
    default:
      return unreachable(event.payload)
  }
}

const onCommand: OnCommand<State, Command, Event> = (state, command) => {
  switch (command.type) {
    case 'enable':
      if (state.type === 'disabled') {
        return [{ type: 'enabled' }]
      }
      return []
    case 'disable':
      if (state.type === 'enabled') {
        return [{ type: 'disabled' }]
      }
      return []
    case 'inc':
      if (state.type === 'enabled') {
        return [{ type: 'inced' }]
      }
      return []
    default:
      return unreachable(command)
  }
}

const onStateChange: OnStateChange<State, Command, State> = pond => {

  const timerFishUpdates: Observable<StateEffect<Command, State>> = mkTimerFishListener(
    pond,
  ).mergeMap(effect =>
    pond
      .observeSelf()
      .take(1)
      .concatMap(state => {
        if (state.type === 'enabled') {
          return [effect]
        } else {
          return []
        }
      }),
  )

  const publishState: Observable<StateEffect<Command, State>> = pond
    .observeSelf()
    .map(state => StateEffect.publish(state))

  return Observable.merge(timerFishUpdates, publishState)
}

const initialState: InitialState<State> = () => ({ state: { type: 'disabled' } })

export const timerFishListenerFishType: FishType<Command, Event, State> = FishType.of<
  State,
  Command,
  Event,
  State
>({
  semantics: Semantics.of('timerListenerFish'),
  initialState,
  onEvent,
  onCommand,
  onStateChange,
})
