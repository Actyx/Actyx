/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { isNode } from '../util'
import { MultiplexedWebsocket } from './multiplexedWebsocket'
import { Event, Events, WsStoreConfig } from './types'

const defaultConfig: WsStoreConfig = {
  url: (isNode && process.env.AX_STORE_URI) || 'ws://localhost:4243/store_api',
}

export const extendDefaultWsStoreConfig = (overrides: Partial<WsStoreConfig> = {}) => ({
  ...defaultConfig,
  ...overrides,
})

export const mkMultiplexer = (config: Partial<WsStoreConfig> = {}): MultiplexedWebsocket => {
  const c: WsStoreConfig = extendDefaultWsStoreConfig(config)

  return new MultiplexedWebsocket(c)
}

/**
 * Randomly interleaves several arrays so that the order within each array is preserved.
 */
export const interleaveRandom = <T>(arrays: ReadonlyArray<ReadonlyArray<T>>): T[] => {
  const length = arrays.reduce((acc, a) => acc + a.length, 0)
  const result: T[] = new Array(length)

  const nonEmpty = arrays.filter(x => x.length > 0)
  const offsets = new Array(nonEmpty.length).fill(0)
  for (let i = 0; i < length; i++) {
    const pick = Math.floor(Math.random() * nonEmpty.length)
    result[i] = nonEmpty[pick][offsets[pick]]
    if (offsets[pick] + 1 === nonEmpty[pick].length) {
      nonEmpty.splice(pick, 1)
      offsets.splice(pick, 1)
    } else {
      offsets[pick]++
    }
  }

  return result
}

// Partition an unordered batch of events into several, where each is internally ordered.
// Will not copy any data if the whole input batch is already sorted.
export const intoOrderedChunks = (batch: Events) => {
  if (batch.length < 2) {
    return [batch]
  }

  const orderedBatches: Events[] = []

  let prev = batch[0]
  let start = 0

  for (let i = 1; i < batch.length; i++) {
    const evt = batch[i]

    if (Event.ord.compare(prev, evt) > 0) {
      orderedBatches.push(batch.slice(start, i))
      start = i
    }

    prev = evt
  }

  if (start === 0) {
    // Everything was sorted already
    orderedBatches.push(batch)
  } else {
    orderedBatches.push(batch.slice(start))
  }

  return orderedBatches
}
