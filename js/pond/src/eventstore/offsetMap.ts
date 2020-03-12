/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { fromNullable } from 'fp-ts/lib/Option'
import * as t from 'io-ts'
import { Psn } from '../types'

export const OffsetMapIO = t.readonly(t.record(t.string, Psn.FromNumber))
/**
 * A offset map stores the high water mark for each source.
 *
 * The value in the psn map is the highest psn seen for this source. Since sequence
 * numbers start with 0, the default value for sources that are not present is -1
 */
export type OffsetMap = t.TypeOf<typeof OffsetMapIO>

const emptyOffsetMap: OffsetMap = {}
const offsetMapLookup = (m: OffsetMap, s: string): Psn => fromNullable(m[s]).getOrElse(Psn.min)

/**
 * Relatively pointless attempt to distinguish between mutable and immutable psnmap
 * See https://github.com/Microsoft/TypeScript/issues/13347 for why this does not help much.
 */
export type OffsetMapBuilder = Record<string, Psn>
export type OffsetMapCompanion = Readonly<{
  empty: OffsetMap
  lookup: (m: OffsetMap, s: string) => Psn
  lookupOrUndefined: (m: OffsetMap, s: string) => Psn | undefined
}>

export const OffsetMap: OffsetMapCompanion = {
  empty: emptyOffsetMap,
  lookup: offsetMapLookup,
  lookupOrUndefined: (m: OffsetMapBuilder, s: string) => m[s],
}
