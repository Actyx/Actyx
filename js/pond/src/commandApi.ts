/* eslint-disable @typescript-eslint/no-explicit-any */
import { Either, left, right } from 'fp-ts/lib/Either'
import { Observable } from 'rxjs'
import { FishName, FishType } from '.'
import { Target } from './types'
import { LogFunction, QueryParams } from './util'

//#region impl
export const enum NodeType {
  Of = 'Of',
  Map = 'Map',
  Chain = 'Chain',
  All = 'All',
  HttpRequest = 'HttpRequest',
  SendCommand = 'SendCommand',
  GetState = 'GetState',
  Log = 'Log',
  UnsafeAsync = 'UnsafeAsync',
}

const mkRequest: (options: RequestOptions) => CommandApi<HttpResponse> = options =>
  mkCommandApi({
    type: NodeType.HttpRequest,
    options,
  })

const httpApi: HttpApi = {
  request: mkRequest,
  get: url => mkRequest({ method: 'GET', url }),
}

const pondApi: PondApi = {
  send: <C>(target: Target<C>) => (command: C) =>
    mkCommandApi({
      type: NodeType.SendCommand,
      command,
      target,
    }),
  peek: <P>(fish: FishType<any, any, P>, name: FishName) =>
    mkCommandApi({
      type: NodeType.GetState,
      fish,
      name,
    }),
}

export type OfNode<T> = {
  type: NodeType.Of
  value: T
}

export type HttpRequestNode = {
  type: NodeType.HttpRequest
  options: RequestOptions
}

export type SendCommandNode<C> = {
  type: NodeType.SendCommand
  command: C
  target: Target<C>
}

export type GetStateNode<P> = {
  type: NodeType.GetState
  fish: FishType<any, any, P>
  name: FishName
}

export type MapNode<T, U> = {
  type: NodeType.Map
  source: CommandApi<T>
  f: (value: T) => U
}

export type LogNode<T> = {
  type: NodeType.Log
  source: CommandApi<T>
  logTarget?: LogFunction
}

export type ChainNode<T, U> = {
  type: NodeType.Chain
  source: CommandApi<T>
  f: (value: T) => CommandApi<U>
}

export type AllNode<T> = {
  type: NodeType.All
  values: ReadonlyArray<CommandApi<T>>
}

export type UnsafeAsyncNode<T> = {
  type: NodeType.UnsafeAsync
  value: Observable<T>
}

export type AstNode =
  | OfNode<any>
  | MapNode<any, any>
  | ChainNode<any, any>
  | AllNode<any>
  | SendCommandNode<any>
  | GetStateNode<any>
  | HttpRequestNode
  | LogNode<any>
  | UnsafeAsyncNode<any>

/**
 * This is NOT a public API. It is only intended for internal exploration of
 * candidate operations to include in the real API later (i.e. provide an ad hoc
 * implementation of something that may later become a properly encoded AstNode).
 * One serious downside is that the testCommandExecutor will always evaluate
 * such nodes to an error result.
 */
export const UnsafeAsync = <T>(value: Observable<T>): CommandApi<T> =>
  mkCommandApi({
    type: NodeType.UnsafeAsync,
    value,
  })

const commandApiPrototype = {
  map<T, U>(this: CommandApi<T>, f: (value: T) => U): CommandApi<U> {
    return mkCommandApi<U>({
      type: NodeType.Map,
      source: this,
      f,
    })
  },
  chain<T, U>(this: CommandApi<T>, f: (value: T) => CommandApi<U>): CommandApi<U> {
    return mkCommandApi<U>({
      type: NodeType.Chain,
      source: this,
      f,
    })
  },
  log<T>(this: CommandApi<T>, logTarget?: LogFunction): CommandApi<T> {
    return mkCommandApi({
      type: NodeType.Log,
      source: this,
      logTarget,
    })
  },
}

export const AstNode: <T>(source: CommandApi<T>) => AstNode = source => source as any

function mkCommandApi<T>(result: AstNode): CommandApi<T> {
  return Object.assign(Object.create(commandApiPrototype), result)
}

const noEvents: CommandApi<ReadonlyArray<never>> = mkCommandApi({
  type: NodeType.Of,
  value: [],
})
export const CommandApi: CommandApiCompanion = {
  of: <U>(value: U): CommandApi<U> =>
    mkCommandApi({
      type: NodeType.Of,
      value,
    }),
  all: <U>(values: ReadonlyArray<CommandApi<U>>): CommandApi<ReadonlyArray<U>> =>
    mkCommandApi({
      type: NodeType.All,
      values,
    }),
  http: httpApi,
  pond: pondApi,
  noEvents,
}
//#endregion
export type CommandApiCompanion = {
  of: <U>(value: U) => CommandApi<U>
  /**
   * Performs a number of event api operations. The executor may perform the operations concurrently.
   */
  all: <U>(values: ReadonlyArray<CommandApi<U>>) => CommandApi<ReadonlyArray<U>>
  http: HttpApi
  pond: PondApi
  /**
   * Constant to use to return no events
   */
  noEvents: CommandApi<ReadonlyArray<never>>
}

/**
 * A value of type CommandApi<T> is a value that has been generated using the command api.
 * It stores an AST of a computation that can be executed by an executor.
 *
 * The functions passed to map and chain are expected to handle errors explicitly. An operation
 * that has a reasonable expectation of failure should return an Either<Error, X> instead of just
 * throwing an exception. The error should be kept through all subsequent stages and used to
 * produce appropriate events (or no events).
 */
export type CommandApi<T> = {
  /**
   * If f throws an error, the error will be logged and the processing of the command stopped.
   */
  map: <U>(f: (value: T) => U) => CommandApi<U>

  /**
   * If f throws an error, the error will be logged and the processing of the command stopped.
   */
  chain: <U>(f: (value: T) => CommandApi<U>) => CommandApi<U>

  /**
   * Log current data to given log function
   */
  log: (f?: LogFunction) => CommandApi<T>
}

/**
 * this is the most powerful type that we are interested in usually when explicitly handling errors
 */
export type CE<E, V> = CommandApi<Either<E, V>>

/**
 * utility methods to lift various functions into the CE type, assuming that you just want to transform
 * the happy path.
 */
export const CE = {
  liftC: <E, U, V>(f: (x: U) => CommandApi<V>) => (value: Either<E, U>): CE<E, V> =>
    value.fold(error => CommandApi.of(left<E, V>(error)), s => f(s).map(x => right<E, V>(x))),
  liftCE: <E, U, V>(f: (x: U) => CE<E, V>) => (value: Either<E, U>): CE<E, V> =>
    value.fold(error => CommandApi.of(left<E, V>(error)), s => f(s)),
}

/**
 * The response in case of a successful request, given as either a string, a buffer or a json object
 * depending on the kind of request that was performed.
 */
export type HttpSuccess = any

export type RequestOptions =
  | {
      method: 'GET'
      url: string
      params?: QueryParams
      headers?: { [key: string]: string }
      responseType?: 'json' | 'txt'
    }
  | {
      method: 'POST'
      url: string
      params?: QueryParams
      headers?: { [key: string]: string }
      data: any
    }

export type Error = any

export type HttpResponse = Either<Error, HttpSuccess>

export type EmptyEventArray = ReadonlyArray<never>

export type HttpApi = {
  /**
   * Execute a simple HTTP request
   */
  request: (options: RequestOptions) => CommandApi<HttpResponse>
  /**
   * Convenience overload for executing simple get requests
   */
  get: (url: string) => CommandApi<HttpResponse>
}

export type PondApi = {
  /**
   * Send a command to a target. This operation is fire-and-forget and will
   * return immediately, even before the command has been processed by the target.
   */
  send: <C>(target: Target<C>) => (command: C) => CommandApi<EmptyEventArray>
  /**
   * Get the current public state of a fish. Note that if you send a command to
   * a fish and then peek its state within one command, there is no guarantee
   * that you will see the effect of the command. In fact, it is most likely
   * that you won't.
   */
  peek: <P>(f: FishType<any, any, P>, n: FishName) => CommandApi<P>
}
