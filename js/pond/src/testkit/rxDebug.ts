/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2020 Actyx AG
 */
/**
 * Debug tools for finding issues with rxjs pipelines
 *
 * DO NOT ever use this in production. This is just meant for debugging.
 */

import { Observable } from '../../node_modules/rxjs'
/**
 * A stateful observable transform that injects concurrency tracking for debugging
 *
 * To track how often an observable is being instantiated at a location,
 * create a single instance of this at top level, and use it with pipe.
 */
export const concurrencyTracker =
  (max?: number) =>
  (id: string) =>
  <T>(x: Observable<T>): Observable<T> => {
    const max1 = max || 1
    if (max1 < 0) {
      return x
    }
    const concurrency: { [key: string]: number } = {}
    return new Observable((subscriber) => {
      const current = concurrency[id] || 0
      if (current + 1 > max1) {
        // eslint-disable-next-line no-debugger
        debugger
      }
      concurrency[id] = current + 1
      const subscription = x.subscribe(subscriber)
      return () => {
        concurrency[id] = (concurrency[id] || 0) - 1
        subscription.unsubscribe()
      }
    })
  }

/**
 * An observable transform that simplifies tracking down stack overflows in rxjs pipelines
 *
 * Insert into rxjs pipeline like this:
 * .pipe(soFinder('processEvents1'))
 * and observe the logs. In case of error propagation you will not be dropped into debugger
 * (e.g. caught exception)
 *
 * Name is just for identification within the debugger.
 */
export const soFinder =
  (name: string) =>
  <T>(x: Observable<T>): Observable<T> => {
    return new Observable((subscriber) => {
      const subscription = x.subscribe({
        next: (value) => {
          try {
            subscriber.next(value)
          } catch (e) {
            console.error(name)
            // eslint-disable-next-line no-debugger
            debugger
          }
        },
        error: (error) => {
          try {
            console.info('soFinder', name, JSON.stringify(error))
            subscriber.error(error)
          } catch (e) {
            console.error(name)
            // eslint-disable-next-line no-debugger
            debugger
          }
        },
        complete: () => {
          try {
            subscriber.complete()
          } catch (e) {
            console.error(name)
            // eslint-disable-next-line no-debugger
            debugger
          }
        },
      })
      return () => {
        try {
          subscription.unsubscribe()
        } catch (e) {
          console.error(name)
          // eslint-disable-next-line no-debugger
          debugger
        }
      }
    })
  }
