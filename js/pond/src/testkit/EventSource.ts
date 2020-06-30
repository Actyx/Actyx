/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Envelope, FishName, Lamport, Source, Timestamp } from '../types'

/**
 * Wraps events in envelopes with increasing sequence number and timestamp for testing
 */
export type EventSource<E> = (event: E) => Envelope<E>

export const EventSource = {
  of: mkEventSource,
}

export function mkEventSource<E>(
  source: Source,
): (event: E, timestamp?: number, name?: FishName) => Envelope<E> {
  let seq = 0

  return (e, t, name) => {
    seq += 1
    return {
      source: {
        ...source,
        name: name || source.name,
      },
      timestamp: t !== undefined ? Timestamp.of(t) : Timestamp.of(seq * 1000),
      lamport: t !== undefined ? Lamport.of(t) : Lamport.of(seq * 1000),
      tags: [],
      payload: e,
    }
  }
}
