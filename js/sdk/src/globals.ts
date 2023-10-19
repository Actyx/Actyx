import { GlobalInternalSymbol } from './v2/utils'

type ActiveRequest = {
  serviceId: string
  payload?: unknown
  time: Date
}

type ActiveRequestGlobals = {
  all: () => ActiveRequest[]
  [GlobalInternalSymbol]: {
    register: (sym: symbol, activeRequest: ActiveRequest) => unknown
    unregister: (sym: symbol) => unknown
  }
}

/**
 * Global object for accessing currently active requests
 * @public
 */
export const activeRequests = ((): ActiveRequestGlobals => {
  const map = new Map<symbol, ActiveRequest>()

  const all: ActiveRequestGlobals['all'] = () => Array.from(map.values())

  const register: ActiveRequestGlobals[GlobalInternalSymbol]['register'] = (sym, req) => {
    map.set(sym, req)
  }

  const unregister: ActiveRequestGlobals[GlobalInternalSymbol]['unregister'] = (sym) => {
    map.delete(sym)
  }

  return {
    all,
    [GlobalInternalSymbol]: {
      register,
      unregister,
    },
  }
})()
