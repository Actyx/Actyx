/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable } from 'rxjs'
import { log } from '../loggers'
import { noop } from './misc'
import { isNode } from './runtime'

export type QueryParams = {
  [arg: string]: any
}
export type RequestOptions = RequestInit & {
  timeout?: number
  params?: QueryParams
}

export type FetchObs = (
  uri: string,
  options: RequestOptions,
  params?: QueryParams,
) => Observable<Response>

const mkAbortController = (options: RequestOptions, onTimeout: () => void) => {
  if (isNode) {
    // https://github.com/bitinn/node-fetch/issues/95
    return { options, timer: undefined, abort: noop }
  }

  const { timeout, ...rest } = options
  const abortController = new AbortController()
  const abort = () => abortController.abort()
  const timer =
    timeout !== undefined && timeout > 0
      ? setTimeout(() => {
          onTimeout()
          abort()
        }, timeout)
      : undefined
  const signal = abortController.signal
  return { options: { ...rest, signal }, timer, abort }
}

export const createQueryString = (data: any) => {
  if (data === undefined) {
    return ''
  }
  const props = Object.keys(data)
    .map(key => {
      const value = data[key]
      if (Array.isArray(value)) {
        return value.map((v: any) => `${key}=${encodeURIComponent(v)}`).join('&')
      }

      return `${key}=${encodeURIComponent(value)}`
    })
    .join('&')

  return `?${props}`
}

/**
 * Executes `fetch` calls and returns a `Observable` that cancels the request when `unsubscribe` is called
 * on its subscription (only supported in the browser). The request is also cancelled when the optionally provided
 * `timeout` config setting is reached.
 *
 * Note that when the response contains a chunked stream, cancellation is no longer possible after the first
 * chunk has been received!
 *
 * This returns a cold observable. The actual request will only initialized once (and every time) the observable
 * is subscribed to.
 */
export const fetchObs: FetchObs = (
  uri: string,
  options: RequestOptions,
  params?: QueryParams,
): Observable<Response> =>
  new Observable(subscription => {
    const { options: withTimeout, timer, abort } = mkAbortController(options, () =>
      log.http.error('timeout when fetching', uri, 'after', options.timeout),
    )
    const uriWithParams = params ? `${uri}${createQueryString(params)}` : uri
    // TODO: come up with a better mechanism
    // the issue with this mechanism is that it is no longer possible to abort
    // streamed (chunked) responses at all once the initial response has been received.
    // so e.g. a streamed download of a 1GB video from IPFS would no longer be abortable,
    // which is not good!
    // But for now this prevents the streamed response for the pubsub sub to be aborted.
    let completed: boolean = false
    fetch(uriWithParams, withTimeout).then(
      response => {
        completed = true
        const ok = response.status >= 200 && response.status < 300
        if (!ok) {
          subscription.error({
            status: 'error',
            error: {
              status: response.status,
              statusText: response.statusText,
            },
          })
        } else {
          subscription.next(response)
          subscription.complete()
        }
      },
      err => {
        completed = true
        subscription.error({
          status: 'networkError',
          error: err,
        })
      },
    )

    const unsubscribe = () => {
      if (timer !== undefined) {
        clearTimeout(timer)
      }
      if (!completed) {
        abort()
      }
    }
    return unsubscribe
  })
