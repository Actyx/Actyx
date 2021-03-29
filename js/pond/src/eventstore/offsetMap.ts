/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { fromNullable } from 'fp-ts/lib/Option'
import * as t from 'io-ts'
import { Offset } from '../types'
import { lookup } from '../util'
import { Event } from './types'

export const OffsetMapIO = t.readonly(t.record(t.string, Offset.FromNumber))
/**
 * A offset map stores the high water mark for each source.
 *
 * The value in the psn map is the highest psn seen for this source. Since sequence
 * numbers start with 0, the default value for sources that are not present is -1
 */
export type OffsetMap = t.TypeOf<typeof OffsetMapIO>

const emptyOffsetMap: OffsetMap = {}
const offsetMapLookup = (m: OffsetMap, s: string): Offset =>
  fromNullable(m[s]).getOrElse(Offset.min)

/**
 * Updates a given psn map with a new event.
 * Note that the events need to be applied in event order
 *
 * @param psnMap the psn map to update. WILL BE MODIFIED IN PLACE
 * @param ev the event to include
 */
const includeEvent = (psnMap: OffsetMapBuilder, ev: Event): OffsetMapBuilder => {
  const { offset, stream } = ev
  const current = lookup(psnMap, stream)
  if (current === undefined || current < offset) {
    psnMap[stream] = offset
  }
  return psnMap
}

/**
 * Relatively pointless attempt to distinguish between mutable and immutable psnmap
 * See https://github.com/Microsoft/TypeScript/issues/13347 for why this does not help much.
 */
export type OffsetMapBuilder = Record<string, Offset>
export type OffsetMapCompanion = Readonly<{
  empty: OffsetMap
  isEmpty: (m: OffsetMap) => boolean
  lookup: (m: OffsetMap, s: string) => Offset
  lookupOrUndefined: (m: OffsetMap, s: string) => Offset | undefined
  update: (m: OffsetMapBuilder, ev: Event) => OffsetMapBuilder
}>

export const OffsetMap: OffsetMapCompanion = {
  empty: emptyOffsetMap,
  isEmpty: m => Object.keys(m).length === 0,
  lookup: offsetMapLookup,
  lookupOrUndefined: (m: OffsetMapBuilder, s: string) => m[s],
  update: includeEvent,
}
