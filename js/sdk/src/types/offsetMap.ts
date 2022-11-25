/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { fromNullable, getOrElse as getOrElseO } from 'fp-ts/lib/Option'
import { lookup } from '../util'
import { Offset, StreamId } from './various'

/**
 * A offset map stores the high water mark for each source.
 *
 * The value in the psn map is the highest psn seen for this source. Since sequence
 * numbers start with 0, the default value for sources that are not present is -1
 *
 * @public
 */
export type OffsetMap = Record<StreamId, Offset>

/**
 * Response to an offsets() call
 * @public
 */
export type OffsetsResponse = {
  /** The current local present, i.e. offsets up to which we can provide events without any gaps. */
  present: OffsetMap

  /** For each stream we still need to download events from, the number of pending events. */
  toReplicate: Record<StreamId, number>
}

const emptyOffsetMap: OffsetMap = {}
/**
 * @internal
 */
export const _offsetMapLookup = (m: OffsetMap, s: string): Offset =>
  getOrElseO(() => Offset.min)(fromNullable(m[s]))

/** Anything with offset on a stream.
 * @public */
export type HasOffsetAndStream = {
  offset: number
  stream: string
}

/**
 * Updates a given psn map with a new event.
 * Note that the events need to be applied in event order
 *
 * @param psnMap the psn map to update. WILL BE MODIFIED IN PLACE
 * @param ev the event to include
 */
const includeEvent = (psnMap: OffsetMapBuilder, ev: HasOffsetAndStream): OffsetMapBuilder => {
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
 * @public
 */
export type OffsetMapBuilder = Record<string, Offset>
/** OffsetMap companion functions.
 * @public */
export type OffsetMapCompanion = {
  empty: OffsetMap
  isEmpty: (m: OffsetMap) => boolean
  lookup: (m: OffsetMap, s: string) => Offset
  lookupOrUndefined: (m: OffsetMap, s: string) => Offset | undefined
  update: (m: OffsetMapBuilder, ev: HasOffsetAndStream) => OffsetMapBuilder
}

/** OffsetMap companion functions.
 * @public */
export const OffsetMap: OffsetMapCompanion = {
  empty: emptyOffsetMap,
  isEmpty: (m) => Object.keys(m).length === 0,
  lookup: _offsetMapLookup,
  lookupOrUndefined: (m: OffsetMapBuilder, s: string) => m[s],
  update: includeEvent,
}
