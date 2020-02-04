/* eslint-disable @typescript-eslint/no-explicit-any */
import { left, right } from 'fp-ts/lib/Either'
import { FishName, FishType } from '..'
import { AstNode, CommandApi, NodeType } from '../commandApi'
import { log } from '../loggers'
import { SendCommand } from '../types'
import { fetchObs, unreachable } from '../util'

export type CommandExecutorConfig = Readonly<{
  sendCommand: <T>(sc: SendCommand<T>) => void
  getState: <P>(f: FishType<any, any, P>, n: FishName) => Promise<P>
}>

const defaultConfig: CommandExecutorConfig = {
  sendCommand: console.log,
  getState: <P>(_f: FishType<any, any, P>, _n: FishName) => {
    /* istanbul ignore next */
    throw new Error()
  },
}

export type ResponseType = 'json' | 'txt'
const parseJson = (response: any) =>
  response.text().then((text: any) => (text ? JSON.parse(text) : text))
const parseTxt = (response: any) => response.text()
const parsePayload = (responseType: ResponseType) => {
  switch (responseType) {
    case 'json':
      return parseJson
    case 'txt':
      return parseTxt
  }
}

export type CommandExecutor = <U>(value: CommandApi<U>) => Promise<U>
export const CommandExecutor = (config?: Partial<CommandExecutorConfig>): CommandExecutor => {
  const config1 = { ...defaultConfig, ...config }
  // TODO: switch to Observable<U>
  const executor: <U>(cmd: CommandApi<U>) => Promise<U> = cmd => {
    const ast = AstNode(cmd)
    switch (ast.type) {
      case NodeType.HttpRequest: {
        const { options } = ast
        switch (options.method) {
          case 'GET': {
            const { url, params, responseType, headers } = options
            return fetchObs(
              url,
              {
                method: 'GET',
                mode: 'cors',
                headers,
              },
              params,
            )
              .map(parsePayload(responseType || 'json'))
              .toPromise()
              .then(right)
              .catch(left)
          }
          case 'POST': {
            const { url, params, data, headers } = options
            return fetchObs(
              url,
              {
                method: 'POST',
                mode: 'cors',
                headers: {
                  'Content-Type': 'application/json',
                  ...headers,
                },
                body: JSON.stringify(data),
              },
              params,
            )
              .map(parsePayload('json'))
              .toPromise()
              .then(right)
              .catch(left)
          }
          /* istanbul ignore next */
          default: {
            return unreachable(options)
          }
        }
      }
      case NodeType.SendCommand: {
        const { command, target } = ast

        config1.sendCommand({ command, target })
        return Promise.resolve([])
      }
      case NodeType.GetState: {
        const { fish, name } = ast
        return config1.getState(fish, name)
      }
      case NodeType.Of: {
        return Promise.resolve(ast.value)
      }
      case NodeType.Map: {
        const { source, f } = ast
        return executor(source).then(f)
      }
      case NodeType.Log: {
        const { source, logTarget } = ast
        return executor(source).then(x => {
          const logger = logTarget || log.pond.debug

          logger(x)
          return x
        })
      }
      case NodeType.Chain: {
        const { source, f } = ast
        return executor(source).then(x => executor(f(x)))
      }
      case NodeType.All: {
        const { values } = ast
        return Promise.all(values.map(executor))
      }
      case NodeType.UnsafeAsync: {
        const { value } = ast
        return value.take(1).toPromise()
      }
    }
  }
  return executor
}
