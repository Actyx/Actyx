/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { AppId, Lamport, NodeId, Offset, Timestamp } from '..'
import { Event, Events } from './types'

export type RawEvent = Readonly<{
  payload: unknown
  timestamp: number
  source: string
  tags?: string[]
}>

export type LastPublished = Readonly<{
  timestamp: number
  psn: number
  sequence: number
}>

export const eventFactory = () => {
  const lastPublishedForSources: Record<string, LastPublished> = {}

  const mkEvent: (raw: RawEvent) => Event = (raw) => {
    const lastPublished = lastPublishedForSources[raw.source]
    // Choosing random starting psns for sources should never change test outcomes.
    const offset = lastPublished ? lastPublished.psn : Math.round(Math.random() * 1000)

    if (lastPublished && raw.timestamp < lastPublished.timestamp) {
      throw new Error('A single source will never timetravel, please review your test scenario.')
    }

    const fullEvent = {
      timestamp: Timestamp.of(raw.timestamp),
      stream: NodeId.of(raw.source),
      lamport: Lamport.of(raw.timestamp),
      offset: Offset.of(offset),
      appId: AppId.of('test'),
      payload: raw.payload,
      tags: (raw.tags || []).concat(['default']),
    }

    lastPublishedForSources[raw.source] = {
      timestamp: raw.timestamp,
      psn: offset + 1,
      sequence: offset + 1,
    }

    return fullEvent
  }

  const mkEvents: (raw: RawEvent[]) => Events = (raw) => raw.map(mkEvent)

  return {
    mkEvent,
    mkEvents,
  }
}

type MkNumberEvent = {
  val: number
  source: string
  tAdd: (t: Timestamp) => Timestamp
}

type MkPadding = {
  numEvents: number
  source: string
  tAdd: (t: Timestamp) => Timestamp
}

export type MkEvent = MkNumberEvent | MkPadding

// eslint-disable-next-line no-prototype-builtins
const isPadding = (e: MkEvent): e is MkPadding => e.hasOwnProperty('numEvents')

const incrementBy = (delta: number) => (t: Timestamp) => Timestamp.of(t + delta)

export const emitter = (source: string) => {
  const r = (val: number) => ({
    val,
    source,
    tAdd: incrementBy(100),
  })

  return r
}

export type Timeline = {
  all: Events
  of: (...sources: string[]) => Events
}

export const mkTimeline = (...events: MkEvent[]): Timeline => {
  const { mkEvent } = eventFactory()

  let t = Timestamp.of(100)
  const timeline: Event[] = []

  for (const e of events) {
    t = e.tAdd(t)

    if (isPadding(e)) {
      for (let i = 0; i < e.numEvents; i++) {
        timeline.push(
          mkEvent({
            payload: 'padding',
            timestamp: t,
            source: e.source,
          }),
        )

        t = e.tAdd(t)
      }
    } else {
      timeline.push(
        mkEvent({
          payload: e.val,
          timestamp: t,
          source: e.source,
        }),
      )
    }
  }

  return {
    all: timeline,
    of: (...sources: string[]) => timeline.filter((ev) => sources.includes(ev.stream)),
  }
}
