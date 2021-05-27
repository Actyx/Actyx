/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { Offset, OffsetMap } from '@actyx/sdk'
import * as immutable from 'immutable'
import { Observable } from 'rxjs'
import * as rx from 'rxjs/operators'

/**
 * All the info we got for a single node
 *
 * This might grow in the future to include things like timestamps
 * @public
 */
export type NodeInfoEntry = Readonly<{
  own?: number
  swarm?: number
}>

/**
 * All the info we got for our device in relation to the swarm
 * @public
 */
export type SwarmInfo = Readonly<{
  nodes: immutable.Map<string, NodeInfoEntry>
}>

/**
 * Mutable swarm counters information.
 * @public
 */
export type CountersMut = {
  /**
   * The pond has it, the swarm doesn't
   */
  own: number
  /**
   * The swarm has it, the pond does not
   */
  swarm: number
  /**
   * Both the pond and the swarm have it
   */
  both: number
}

/**
 * Immutable swarm counters information.
 * @public
 */
export type Counters = Readonly<CountersMut>

/**
 * Summary of swarm info
 * @public
 */
export type SwarmSummary = Readonly<{
  info: SwarmInfo
  sources: Counters
  events: Counters
}>

const emptySwarmInfo: SwarmInfo = { nodes: immutable.Map() }

const addPsnMap = (current: SwarmInfo, m: SourcedOffsetMap): SwarmInfo => {
  const nodes = Object.entries(m.roots).reduce((agg, [k, v]) => {
    const entry: NodeInfoEntry = agg.get(k, {})
    const currentPsn = entry[m.from] || Offset.of(-1)
    return v > currentPsn ? agg.set(k, { ...entry, [m.from]: v }) : agg
  }, current.nodes)
  return { nodes }
}

export const toSwarmSummary = (info: SwarmInfo): SwarmSummary => {
  const sources: CountersMut = { own: 0, swarm: 0, both: 0 }
  const events: CountersMut = { own: 0, swarm: 0, both: 0 }
  info.nodes.forEach(({ own, swarm }) => {
    if (own !== undefined) {
      if (swarm !== undefined) {
        sources.both++
        const min = Math.min(own, swarm)
        const max = Math.max(own, swarm)
        events.both += min
        events.own += max - min
      } else {
        sources.own++
        events.own += own
      }
    } else {
      if (swarm !== undefined) {
        events.swarm += swarm
        sources.swarm++
      }
    }
  })
  return {
    info,
    sources,
    events,
  }
}

type From = 'own' | 'swarm'
type SourcedOffsetMap = Readonly<{
  from: From
  roots: OffsetMap
}>

const addOrigin = (from: From) =>
  rx.map<OffsetMap, SourcedOffsetMap>(roots => ({
    from,
    roots,
  }))

export const swarmState = (
  own: Observable<OffsetMap>,
  pubSub: Observable<OffsetMap>,
): Observable<SwarmInfo> =>
  Observable.merge(own.pipe(addOrigin('own')), pubSub.pipe(addOrigin('swarm'))).scan(
    addPsnMap,
    emptySwarmInfo,
  )
const emptySwarmSummary = toSwarmSummary(emptySwarmInfo)

/** SwarmSummary associated functions. @public */
export const SwarmSummary = {
  empty: emptySwarmSummary,
  fromSwarmInfo: toSwarmSummary,
}
