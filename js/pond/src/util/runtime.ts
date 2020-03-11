/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
// utilities that are specific to the runtime / execution environment

// true if we are probably running on nodejs
export const isNode: boolean = process && process.toString() === '[object process]'

// getting memory usage in bytes
export function getMemoryUsage(): { [key: string]: number } {
  try {
    if (isNode) {
      // deconstruct process.memoryUsage() to change the names of the properties
      const {
        heapUsed: usedJSHeapSize,
        heapTotal: totalJSHeapSize,
        external: externalSize,
        rss: residentSetSize,
      } = process.memoryUsage()
      return { usedJSHeapSize, totalJSHeapSize, externalSize, residentSetSize }
    } else {
      // deconstruct window.performance.memory since it is not enumerable
      const {
        usedJSHeapSize,
        totalJSHeapSize,
        jsHeapSizeLimit,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
      } = (window.performance as any).memory
      return {
        usedJSHeapSize,
        totalJSHeapSize,
        jsHeapSizeLimit,
      }
    }
  } catch (_) {
    /* ignore the error */
  }
  return {}
}
