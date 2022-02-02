/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
import * as t from 'io-ts'
import { isNumber, isString } from './functions'
import { AppId, Lamport, Milliseconds, NodeId, Offset, StreamId, Timestamp } from './various'
import { map as mapE } from 'fp-ts/lib/Either'

/**
 *
 * Wire format types.
 * Not open to the public, just for validation during testing.
 */

// Hide the io-ts stuff from the outside world
/** @internal */
export const Codecs = {
  NodeId: new t.Type<NodeId, string>(
    'NodeIdFromString',
    (x): x is NodeId => isString(x),
    (x, c) => mapE((s) => s as NodeId)(t.string.validate(x, c)),
    (x) => x,
  ),

  StreamId: new t.Type<StreamId, string>(
    'StreamIdFromString',
    (x): x is StreamId => isString(x),
    (x, c) => mapE((s) => s as StreamId)(t.string.validate(x, c)),
    (x) => x,
  ),

  AppId: new t.Type<AppId, string>(
    'AppIdFromString',
    (x): x is AppId => isString(x),
    (x, c) => mapE((s) => s as AppId)(t.string.validate(x, c)),
    (x) => x,
  ),

  Lamport: new t.Type<Lamport, number>(
    'LamportFromNumber',
    (x): x is Lamport => isNumber(x),
    (x, c) => mapE((s) => Lamport.of(s as number))(t.number.validate(x, c)),
    (x) => x,
  ),

  Offset: new t.Type<Offset, number>(
    'OffsetFromNumber',
    (x): x is Offset => isNumber(x),
    (x, c) => mapE((s) => s as Offset)(t.number.validate(x, c)),
    (x) => x,
  ),

  Timestamp: new t.Type<Timestamp, number>(
    'TimestampFromNumber',
    (x): x is Timestamp => isNumber(x),
    (x, c) => mapE((s) => s as Timestamp)(t.number.validate(x, c)),
    (x) => x,
  ),

  Milliseconds: new t.Type<Milliseconds, number>(
    'MilisecondsFromString',
    (x): x is Milliseconds => isNumber(x),
    (x, c) => mapE((m) => Milliseconds.of(m as number))(t.number.validate(x, c)),
    (x) => x,
  ),
}

/**
 * Triple that Actyx events are sorted and identified by.
 * Wire format.
 *
 * @internal
 */
export const EventKeyIO = t.readonly(
  t.type({
    lamport: Codecs.Lamport,
    offset: Codecs.Offset,
    stream: Codecs.StreamId,
  }),
)

/** OffsetMap serialization format. @internal */
export const OffsetMapIO = t.readonly(t.record(t.string, Codecs.Offset))
