/* eslint-disable @typescript-eslint/no-explicit-any */
import { Either, fromNullable, left } from 'fp-ts/lib/Either'
import { pathOr } from 'ramda'
import { AstNode, CommandApi, NodeType } from '../commandApi'
import {
  AsyncCommandResult,
  CommandResult,
  FishTypeImpl,
  SourceId,
  SyncCommandResult,
} from '../types'
import { createQueryString, unreachable } from '../util'

//#region impl
export const TestResult = {
  all: <E, T>(values: ReadonlyArray<TestResult<E, T>>): TestResult<E, ReadonlyArray<T>> => {
    const result = values.map(x => x.result)
    const effects = ([] as ReadonlyArray<E>).concat(...values.map(x => x.effects))
    return { result, effects }
  },
  of: <E, T>(value: T, effects: ReadonlyArray<E>): TestResult<E, T> => ({ result: value, effects }),
  map: <E, T, U>(value: TestResult<E, T>, f: (value: T) => U): TestResult<E, U> => ({
    effects: value.effects,
    result: f(value.result),
  }),
  chain: <E, T, U>(
    value: TestResult<E, T>,
    f: (value: T) => TestResult<E, U>,
  ): TestResult<E, U> => {
    const { effects, result } = f(value.result)
    return { effects: value.effects.concat(effects), result }
  },
}
const defaultConfig: TestCommandExecutorConfig = {
  fishStates: {},
  httpGet: {},
  httpPost: {},
  followCommands: {},
}

const mkTestCommandExecutor = (config: Partial<TestCommandExecutorConfig>): TestCommandExecutor => {
  const config1 = { ...defaultConfig, ...config }
  const executor: <T>(cmd: CommandApi<T>) => TestResult<any, T> = cmd => {
    const ast = AstNode(cmd)
    switch (ast.type) {
      case NodeType.Of: {
        return TestResult.of(ast.value, [])
      }
      case NodeType.Map: {
        const { source, f } = ast
        return TestResult.map(executor(source), f)
      }
      case NodeType.Chain: {
        const { source, f } = ast
        return TestResult.chain(executor(source), x => executor(f(x)))
      }
      case NodeType.Log: {
        const { source, logTarget } = ast
        const result = executor(source)
        const text = JSON.stringify(result.result)
        // should this be considered an effect at all?
        return TestResult.of(result.result, [...result.effects, `log ${logTarget} ${text}`])
      }
      case NodeType.All: {
        const { values } = ast
        return TestResult.all(values.map(executor))
      }
      case NodeType.SendCommand: {
        const { command, target } = ast
        const fishType = config1.followCommands[target.semantics.semantics]
        if (fishType) {
          const state =
            config1.fishStates[target.name] ||
            fishType.initialState(target.name, SourceId.of('FakeSourceId'))
          const follow = mkTestCommandExecution(config1)
          const result = follow(fishType.onCommand(state, command))
          const c = JSON.stringify(command)
          return TestResult.of(
            [],
            [
              [
                `following pond.send(${target.semantics.semantics}, ${target.name})(${c}) = `,
                result,
              ],
            ],
          )
        } else {
          const c = JSON.stringify(command)
          return TestResult.of(
            [],
            [`pond.send(${target.semantics.semantics}, ${target.name})(${c})`],
          )
        }
      }
      case NodeType.GetState: {
        const { fish, name } = ast
        const state = pathOr(undefined, [fish.semantics, name])(config1.fishStates)

        if (state === undefined) {
          throw new Error(`missing state data for ${fish.semantics}/${name}`)
        }
        return TestResult.of(state, [`pond.state(${fish.semantics}, ${name})`])
      }
      case NodeType.HttpRequest: {
        const { options } = ast
        switch (options.method) {
          case 'GET': {
            const { url, params } = options
            const query = params ? `${url}${createQueryString(params)}` : url
            const value = config1.httpGet[query]
            const result = fromNullable(`${url} not found`)(value)
            return TestResult.of(result, [`GET request to ${query}`])
          }
          case 'POST': {
            const { url, params, data } = options
            const query = params ? `${url}${createQueryString(params)}` : url
            const value = config1.httpPost[query]
            const effect = `POST request to ${query}: ${JSON.stringify(data)}`
            return value === undefined
              ? TestResult.of(left(`${url} not found`), [effect])
              : TestResult.of(value, [effect])
          }
        }
        /* istanbul ignore next */
        return unreachable(options)
      }
      case NodeType.UnsafeAsync: {
        return TestResult.of(left(`UnsafeAsync cannot be tested`), [])
      }
    }
  }
  return executor
}

// helper method to deal with the fact that onCommand returns some weird discriminated union currently
const mkTestCommandExecution = (config: Partial<TestCommandExecutorConfig>) => {
  const executor = mkTestCommandExecutor(config)
  return <E>(cr: CommandResult<E>): TestResult<any, any> => {
    return CommandResult.fold<any, TestResult<any, any>>(cr)({
      sync: () => {
        throw new Error(`Not an async command result ${cr}`)
      },
      none: () => {
        throw new Error(`Not an async command result ${cr}`)
      },
      async: acmd => executor(acmd),
    })
  }
}
//#endregion
/**
 * Data type that captures both the effects and the result of executing a CommandApi
 */
export type TestResult<E, T> = {
  result: T
  effects: ReadonlyArray<E>
}
/**
 * Configuration for the test executor
 */
export type TestCommandExecutorConfig = Readonly<{
  /**
   * Content of simulated web for get requests
   */
  httpGet: { [url: string]: any }
  /**
   * Content of simulated responses for post requests
   */
  httpPost: { [url: string]: Either<any, any> | undefined }
  /**
   * Fish states
   */
  fishStates: { [sem: string]: { [name: string]: any } }
  followCommands: {
    [sem: string]: FishTypeImpl<any, any, any, any>
  }
}>
/**
 * A command executor that executes the CommandApi synchronously and deterministically.
 * This is useful for testing synchronously.
 */
export type TestCommandExecutor = <T>(value: CommandApi<T>) => TestResult<any, T>
export const TestCommandExecutor: (
  config: Partial<TestCommandExecutorConfig>,
) => TestCommandExecutor = mkTestCommandExecutor
export const TestCommandExecution: (
  config: Partial<TestCommandExecutorConfig>,
) => <E>(
  cr: SyncCommandResult<E> | AsyncCommandResult<E>,
) => TestResult<ReadonlyArray<E>, any> = mkTestCommandExecution
