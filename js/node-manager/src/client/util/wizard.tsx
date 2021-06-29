import React, { Reducer, useReducer, useState } from 'react'
import { Either, isRight } from 'fp-ts/lib/Either'

export type WizardInput<I> = React.FC<{
  execute: (input: I) => void
  executing: boolean
}>
export type WizardSuccess<S> = React.FC<{ result: S; restart: () => void }>
export type WizardFailure<F> = React.FC<{ reason: F; restart: () => void }>

interface Props<I, S, F> {
  input: WizardInput<I>
  success: WizardSuccess<S>
  failure: WizardFailure<F>
  execute: (input: I) => Promise<Either<F, S>>
}

interface StateT<T> {
  s: T
}
type State<I, S, F> =
  | (StateT<S> & { key: 's' })
  | (StateT<F> & { key: 'f' })
  | { key: 'i' }
  | { key: 'e' }
type Action<I, S, F> = State<I, S, F>

function reducer<I, S, F>(_state: State<I, S, F>, action: Action<I, S, F>): State<I, S, F> {
  switch (action.key) {
    case 'i': {
      return { key: 'i' }
    }
    case 'e': {
      return { key: 'e' }
    }
    case 'f': {
      return { ...action }
    }
    case 's': {
      return { ...action }
    }
  }
}

export function Wizard<Input, Success, Failure>({
  input,
  success,
  failure,
  execute,
}: Props<Input, Success, Failure>) {
  const INITIAL_STATE: State<Input, Success, Failure> = { key: 'i' }
  const [state, dispatch] = useReducer<
    Reducer<State<Input, Success, Failure>, Action<Input, Success, Failure>>
  >(reducer, INITIAL_STATE)
  const I = input
  const S = success
  const F = failure

  const wrappedExecute = (i: Input) => {
    ;(async () => {
      dispatch({ key: 'e' })
      const r = await execute(i)
      if (isRight(r)) {
        dispatch({ key: 's', s: r.right })
        return
      }
      dispatch({ key: 'f', s: r.left })
    })()
  }

  const restart = () => dispatch({ key: 'i' })

  switch (state.key) {
    case 'i':
      return <I execute={wrappedExecute} executing={false} />
    case 'e':
      // This is a simplification so that you can just use three components; just
      // don't call execute when executing
      // eslint-disable-next-line @typescript-eslint/no-empty-function
      return <I execute={() => {}} executing={true} />
    case 's':
      return <S restart={restart} result={state.s} />
    case 'f':
      return <F restart={restart} reason={state.s} />
  }
}
