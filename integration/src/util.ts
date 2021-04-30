import { runOnEvery } from './infrastructure/hosts'
import { ActyxNode } from './infrastructure/types'

const getHttpApi = (x: ActyxNode) => x._private.httpApiOrigin
export const run = <T>(f: (httpApi: string) => Promise<T>): Promise<T[]> =>
  runOnEvery((node) => f(getHttpApi(node)))
