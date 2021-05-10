/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

import { Observable } from 'rxjs'

export type ReconnectStrategy = (x: Observable<any>) => Observable<any>

function logReconnectAttempt(delayMs: number, attempt: number, name: string): void {
  console.info(
    'Connection issue %s, will retry in %d s (Attempt: %d).',
    name,
    delayMs / 1000,
    attempt,
  )
}

type RetryConfig = {
  delayMs?: number
  attempts?: number
  name?: string
}

const retry: (config?: RetryConfig) => ReconnectStrategy = config => e => {
  const delayMs: number = (config && config.delayMs) || 1000
  const name: string = (config && config.name && ` connecting to ${config.name}`) || ''
  const attempts = config && config.attempts
  if (attempts && attempts > 0) {
    return e
      .map((v, i) => {
        const attempt: number = i + 1
        if (attempt > attempts) throw new Error(`Giving up after ${attempts} retries!`)
        logReconnectAttempt(delayMs, attempt, name)
        return v
      })
      .delay(delayMs)
  }
  return e.delay(delayMs)
}

type ExponentialBackoffConfig = {
  minDelay?: number
  maxDelay?: number
  attempts?: number
  name?: string
}

const exponentialBackoff: (
  config?: ExponentialBackoffConfig,
) => ReconnectStrategy = config => e => {
  const minDelay = (config && config.minDelay) || 1000
  const maxDelay = (config && config.maxDelay) || 60000
  const attempts = (config && config.attempts) || 0
  const name: string = (config && config.name && ` connecting to ${config.name}`) || ''
  return e.flatMap((v, i) => {
    const attempt: number = i + 1
    if (attempts > 0 && attempt > attempts) throw new Error(`Giving up after ${attempts} retries!`)
    const delayMs = Math.min(minDelay * 2 ** (attempt - 1), maxDelay)
    logReconnectAttempt(delayMs, attempt, name)
    return Observable.of(v).delay(delayMs)
  })
}

const reconnectStrategies = {
  exponentialBackoff,
  retry,
}

export default reconnectStrategies
