/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import { fromNullable } from 'fp-ts/lib/Option'
import { lookup } from '../util'

/**
 * An Actyx node id.
 * @public
 */
export type NodeId = string
const mkNodeId = (text: string): NodeId => text as NodeId
const randomBase58: (digits: number) => string = (digits: number) => {
  const base58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'.split('')

  let result = ''
  let char

  while (result.length < digits) {
    char = base58[(Math.random() * 57) >> 0]
    result += char
  }
  return result
}

/**
 * `SourceId` associated functions.
 * @public
 */
export const NodeId = {
  /**
   * Creates a NodeId from a string
   */
  of: mkNodeId,
  /**
   * Creates a random SourceId with the given number of digits
   */
  random: (digits?: number) => mkNodeId(randomBase58(digits || 11)),

  streamNo: (nodeId: NodeId, num: number) => nodeId + '-' + num,
}

/**
 * An Actyx stream id.
 * @public
 */
export type StreamId = string
const mkStreamId = (text: string): StreamId => text as StreamId

/**
 * `SourceId` associated functions.
 * @public
 */
export const StreamId = {
  /**
   * Creates a StreamId from a string
   */
  of: mkStreamId,
  /**
   * Creates a random StreamId off a random NodeId.
   */
  random: () => NodeId.streamNo(mkNodeId(randomBase58(11)), Math.floor(Math.random() * 100)),
}

/** Offset within an Actyx event stream. @public */
export type Offset = number
const mkOffset = (n: number): Offset => n as Offset

/** Functions related to Offsets. @public */
export const Offset = {
  of: mkOffset,
  zero: mkOffset(0),
  /**
   * A value that is below any valid Offset
   */
  min: mkOffset(-1),
  /**
   * A value that is above any valid Offset
   */
  max: mkOffset(Number.MAX_SAFE_INTEGER),
}

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
const offsetMapLookup = (m: OffsetMap, s: string): Offset =>
  fromNullable(m[s]).getOrElse(Offset.min)

/** Anything with offset on a stream. @public */
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
/** OffsetMap companion functions. @public */
export type OffsetMapCompanion = Readonly<{
  empty: OffsetMap
  isEmpty: (m: OffsetMap) => boolean
  lookup: (m: OffsetMap, s: string) => Offset
  lookupOrUndefined: (m: OffsetMap, s: string) => Offset | undefined
  update: (m: OffsetMapBuilder, ev: HasOffsetAndStream) => OffsetMapBuilder
}>

/** OffsetMap companion functions. @public */
export const OffsetMap: OffsetMapCompanion = {
  empty: emptyOffsetMap,
  isEmpty: m => Object.keys(m).length === 0,
  lookup: offsetMapLookup,
  lookupOrUndefined: (m: OffsetMapBuilder, s: string) => m[s],
  update: includeEvent,
}
